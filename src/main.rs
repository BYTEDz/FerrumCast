// src/main.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

mod config;
mod input;
mod ipc;
mod loc;
mod pipeline;
mod platform;
mod stream;

use anyhow::Result;
use std::sync::Arc;
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
}

const VERSION: &str = env!("FERRUMCAST_BUILD_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("ferrumcast {}", VERSION);
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware();
        }

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                use std::os::windows::ffi::OsStrExt;

                unsafe {
                    let wide: Vec<u16> = exe_dir
                        .as_os_str()
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    SetDllDirectoryW(wide.as_ptr());
                }

                let current_path = std::env::var("PATH").unwrap_or_default();
                let new_path = format!("{};{}", exe_dir.display(), current_path);
                unsafe {
                    std::env::set_var("PATH", &new_path);
                    std::env::set_var("GST_PLUGIN_PATH", exe_dir);
                    std::env::set_var("GST_PLUGIN_SYSTEM_PATH", "");
                    std::env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", "");

                    let cache_path = exe_dir.join("gst-registry.bin");
                    std::env::set_var("GST_REGISTRY", &cache_path);
                    std::env::set_var("GST_REGISTRY_1_0", &cache_path);
                }

                let local_scanner = exe_dir.join("gst-plugin-scanner.exe");
                if local_scanner.exists() {
                    unsafe {
                        std::env::set_var("GST_PLUGIN_SCANNER", &local_scanner);
                    }
                }
            }
        }
    }

    if args.iter().any(|arg| arg == "--probe") {
        gstreamer::init().expect("Failed to initialize gstreamer");
        let caps = pipeline::PipelineBuilder::probe_capabilities();
        println!("{}", serde_json::to_string(&caps).unwrap());
        return Ok(());
    }

    unsafe {
        std::env::set_var("NICE_DISABLE_UPNP", "1");
    }

    let subscriber = FmtSubscriber::builder()
        .with_writer(std::io::stderr)
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("{}: v{}", loc::MSG_STARTING_ENGINE, VERSION);

    let (outbound_tx, _outbound_rx) = tokio::sync::broadcast::channel(32);

    gstreamer::init().expect("Failed to initialize gstreamer");

    let caps = Arc::new(pipeline::PipelineBuilder::probe_capabilities());
    info!("available encoders: {:?}", caps.encoders);

    let config_store = Arc::new(config::ConfigStore::new_from_args());
    let initial_cfg = config_store.get();

    #[cfg(target_os = "linux")]
    let initial_token = {
        if initial_cfg.token.is_some() {
            initial_cfg.token.clone()
        } else {
            let token_path = platform::linux::LinuxBackend::resolve_token_file_path();
            std::fs::read_to_string(&token_path).ok()
        }
    };

    #[cfg(not(target_os = "linux"))]
    let initial_token: Option<String> = None;

    let platform_backend = platform::create_platform_backend(
        initial_token,
        outbound_tx.clone(),
        initial_cfg.audio_only,
    )
    .await?;

    #[cfg(target_os = "linux")]
    let ipc_path = "/tmp/ferrumcast.sock";
    #[cfg(target_os = "windows")]
    let ipc_path = r"\\.\pipe\ferrumcast";

    #[cfg(target_os = "linux")]
    let _ = std::fs::remove_file(ipc_path);

    info!("binding IPC to {}", ipc_path);
    let server = Arc::new(ipc::IpcServer::new(ipc_path));

    let enc = pipeline::encoders::resolve_encoder(&initial_cfg.encoder, &caps);
    let pipeline_str = pipeline::PipelineBuilder::build_pipeline(
        &initial_cfg,
        enc.as_ref(),
        platform_backend.as_ref(),
    );

    info!("pipeline: {}", pipeline_str);

    let stream_manager = match stream::StreamManager::new(
        &pipeline_str,
        platform_backend.clone(),
        outbound_tx.clone(),
    ) {
        Ok(m) => Arc::new(m),
        Err(e) => {
            error!("failed to init stream: {}", e);
            return Err(e);
        }
    };

    if let Err(e) = stream_manager.start() {
        error!("pipeline start failed: {}", e);
    }

    let stream_c = stream_manager.clone();
    let config_c = config_store.clone();
    let caps_c = Arc::clone(&caps);
    let outbound_tx_c = outbound_tx.clone();
    let platform_backend_c = Arc::clone(&platform_backend);

    info!("{}", loc::MSG_ENGINE_READY);

    let outbound_tx_server = outbound_tx_c.clone();
    let _server_task = tokio::spawn(async move {
        if let Err(e) = server
            .run(
                move |msg| {
                    let stream = stream_c.clone();
                    let config = config_c.clone();
                    let caps = caps_c.clone();
                    let tx = outbound_tx_c.clone();
                    let platform = platform_backend_c.clone();
                    async move {
                        match msg {
                            ipc::InboundMessage::Control(ipc::ControlMessage::StopStream) => {
                                info!("stopping pipeline (engine stays alive)");
                                let _ = stream.stop();
                            }
                            ipc::InboundMessage::Control(ipc::ControlMessage::RestartPipeline(cfg)) => {
                                info!(
                                    "restarting pipeline via IPC: host={} encoder={:?} audio_only={}",
                                    cfg.client_host, cfg.encoder, cfg.audio_only
                                );
                                config.set(cfg.clone());

                                let enc = pipeline::encoders::resolve_encoder(&cfg.encoder, &caps);
                                let pipeline_str = pipeline::PipelineBuilder::build_pipeline(
                                    &cfg,
                                    enc.as_ref(),
                                    platform.as_ref(),
                                );

                                info!("new pipeline: {}", pipeline_str);
                                match stream.restart_pipeline(&pipeline_str) {
                                    Ok(_) => {
                                        let _ = tx.send(ipc::OutboundMessage::ConfigAck {
                                            active_encoder: stream.active_encoder(),
                                        });
                                    }
                                    Err(e) => {
                                        error!("pipeline restart failed: {}", e);
                                        let _ = tx.send(ipc::OutboundMessage::StreamError {
                                            message: format!("restart failed: {}", e),
                                        });
                                    }
                                }
                            }
                            ipc::InboundMessage::Control(ipc::ControlMessage::ConfigureStream(cfg)) => {
                                info!("stream config updated: bitrate={}kbps", cfg.bitrate);
                                config.set(cfg.clone());
                                if let Err(e) = stream.update_bitrate(cfg.bitrate) {
                                    error!("Failed to update bitrate dynamically: {}", e);
                                }
                                let _ = tx.send(ipc::OutboundMessage::ConfigAck {
                                    active_encoder: stream.active_encoder(),
                                });
                            }
                            ipc::InboundMessage::Control(ipc::ControlMessage::SwitchDisplay { direction }) => {
                                let mut cfg = config.get();
                                let prev_index = cfg.monitor_index;

                                if direction == "next" {
                                    cfg.monitor_index = cfg.monitor_index.saturating_add(1);
                                } else if direction == "prev" {
                                    cfg.monitor_index = cfg.monitor_index.saturating_sub(1);
                                }

                                if cfg.monitor_index != prev_index {
                                    info!("{}: {} -> {}", loc::MSG_MONITOR_SWITCHING, prev_index, cfg.monitor_index);
                                    config.set(cfg.clone());

                                    let enc = pipeline::encoders::resolve_encoder(&cfg.encoder, &caps);
                                    let pipeline_str = pipeline::PipelineBuilder::build_pipeline(
                                        &cfg,
                                        enc.as_ref(),
                                        platform.as_ref(),
                                    );

                                    info!("restarting pipeline for monitor switch: {}", pipeline_str);
                                    if let Err(e) = stream.restart_pipeline(&pipeline_str) {
                                        error!("monitor switch restart failed: {}. Falling back to monitor 0.", e);
                                        if cfg.monitor_index != 0 {
                                            cfg.monitor_index = 0;
                                            config.set(cfg.clone());
                                            let fallback_pipe = pipeline::PipelineBuilder::build_pipeline(
                                                &cfg,
                                                enc.as_ref(),
                                                platform.as_ref(),
                                            );
                                            let _ = stream.restart_pipeline(&fallback_pipe);
                                        }
                                    }
                                }
                            }
                            ipc::InboundMessage::Control(ipc::ControlMessage::GetCapabilities) => {
                                let _ = tx.send(ipc::OutboundMessage::CapabilitiesResponse(
                                    (*caps).clone(),
                                ));
                            }
                            ipc::InboundMessage::Control(ipc::ControlMessage::ForceKeyframe) => {
                                let _ = stream.force_keyframe();
                            }
                            ipc::InboundMessage::MouseInput(ref input) => {
                                platform.handle_mouse_input(input);
                            }
                        }
                    }
                },
                outbound_tx_server,
            )
            .await
        {
            error!("IPC server error: {}", e);
        }
    });

    tokio::signal::ctrl_c().await?;
    info!("{}", loc::MSG_ENGINE_SHUTDOWN);
    let _ = stream_manager.stop();
    Ok(())
}
