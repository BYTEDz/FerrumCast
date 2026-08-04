mod config;
#[cfg(target_os = "windows")]
mod gdi_capture;
mod input;
mod ipc;
mod pipeline;
mod stream;

#[cfg(target_os = "linux")]
mod portal;

use anyhow::Result;
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

use std::sync::Arc;

#[cfg(target_os = "linux")]
fn get_token_file_path() -> std::path::PathBuf {
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(config_home).join("ferrumcast.token")
    } else if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home).join(".config").join("ferrumcast.token")
    } else {
        std::path::PathBuf::from("/tmp/ferrumcast.token")
    }
}

const VERSION: &str = env!("FERRUMCAST_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(target_os = "windows")]
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware();
    }

    unsafe {
        std::env::set_var("NICE_DISABLE_UPNP", "1");
    }
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("starting ferrumcast engine v{}", VERSION);

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--probe") {
        gstreamer::init().expect("Failed to initialize gstreamer");
        let caps = pipeline::PipelineBuilder::probe_capabilities();
        println!("{}", serde_json::to_string(&caps).unwrap());
        std::process::exit(0);
    }

    let (outbound_tx, _outbound_rx) = tokio::sync::broadcast::channel(32);

    #[cfg(target_os = "windows")]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let local_plugins = exe_dir.join("lib/gstreamer-1.0");
                let local_scanner = exe_dir.join("libexec/gstreamer-1.0/gst-plugin-scanner.exe");
                if local_plugins.exists() {
                    unsafe {
                        std::env::set_var("GST_PLUGIN_PATH", &local_plugins);
                        std::env::set_var("GST_PLUGIN_SCANNER", &local_scanner);
                    }
                }

                if let Some(path) = std::env::var_os("PATH") {
                    let mut paths = std::env::split_paths(&path).collect::<Vec<_>>();
                    paths.insert(0, exe_dir.to_path_buf());
                    if let Ok(new_path) = std::env::join_paths(paths) {
                        unsafe {
                            std::env::set_var("PATH", new_path);
                        }
                    }
                }
            }
        }
    }

    gstreamer::init().expect("Failed to initialize gstreamer");

    let caps = Arc::new(pipeline::PipelineBuilder::probe_capabilities());
    info!("available encoders: {:?}", caps.encoders);

    let config_store = Arc::new(config::ConfigStore::new_from_args());

    #[cfg(target_os = "linux")]
    let token_path = get_token_file_path();

    #[cfg(target_os = "linux")]
    let initial_token = {
        let cfg = config_store.get();
        if cfg.token.is_some() {
            cfg.token
        } else {
            std::fs::read_to_string(&token_path).ok()
        }
    };

    #[cfg(target_os = "linux")]
    let ipc_path = "/tmp/ferrumcast.sock";
    #[cfg(target_os = "windows")]
    let ipc_path = r"\\.\pipe\ferrumcast";

    #[cfg(target_os = "linux")]
    let _ = std::fs::remove_file(ipc_path);

    info!("binding IPC to {}", ipc_path);
    let server = Arc::new(ipc::IpcServer::new(ipc_path));

    #[cfg(target_os = "linux")]
    let portal_capture = if pipeline::PipelineBuilder::is_wayland() {
        match portal::request_screencast(initial_token, Some(outbound_tx.clone())).await {
            Ok(c) => {
                if let Some(ref t) = c.restore_token {
                    info!("persisting portal token to {:?}", token_path);
                    if let Some(parent) = token_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&token_path, t);
                }
                Some(c)
            }
            Err(e) => {
                error!("portal failed: {}. falling back to test src.", e);
                None
            }
        }
    } else {
        None
    };

    #[cfg(target_os = "linux")]
    let portal_capture_arc = portal_capture.map(Arc::new);

    let platform_ctx = Arc::new(pipeline::PlatformContext {
        #[cfg(target_os = "linux")]
        portal_info: portal_capture_arc.as_ref().map(|c| (c.node_id, c.fd)),
        #[cfg(target_os = "linux")]
        portal_capture: portal_capture_arc.clone(),
    });

    let initial_cfg = config_store.get();
    let enc = pipeline::encoders::resolve_encoder(&initial_cfg.encoder, &caps);
    let pipeline_str =
        pipeline::PipelineBuilder::build_pipeline(&initial_cfg, enc.as_ref(), &platform_ctx);

    info!("pipeline: {}", pipeline_str);

    let stream_manager = match stream::StreamManager::new(&pipeline_str, outbound_tx.clone()) {
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
    let platform_ctx_c = Arc::clone(&platform_ctx);

    info!("engine ready");

    let outbound_tx_server = outbound_tx_c.clone();
    let _server_task = tokio::spawn(async move {
        if let Err(e) = server
            .run(
                move |msg| {
                    let stream = stream_c.clone();
                    let config = config_c.clone();
                    let caps = caps_c.clone();
                    let tx = outbound_tx_c.clone();
                    let platform_ctx = platform_ctx_c.clone();
                    async move {
                        match msg {
                            ipc::InboundMessage::Control(ipc::ControlMessage::StopStream) => {
                                info!("stopping pipeline (engine stays alive)");
                                let _ = stream.stop();
                            }
                            ipc::InboundMessage::Control(ipc::ControlMessage::RestartPipeline(cfg)) => {
                                info!(
                                    "restarting pipeline via IPC: host={} encoder={:?}",
                                    cfg.client_host, cfg.encoder
                                );
                                config.set(cfg.clone());

                                let enc = pipeline::encoders::resolve_encoder(&cfg.encoder, &caps);
                                let pipeline_str = pipeline::PipelineBuilder::build_pipeline(
                                    &cfg,
                                    enc.as_ref(),
                                    &platform_ctx,
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
                                    info!("Switching monitor from {} to {}", prev_index, cfg.monitor_index);
                                    config.set(cfg.clone());

                                    let enc = pipeline::encoders::resolve_encoder(&cfg.encoder, &caps);
                                    let pipeline_str = pipeline::PipelineBuilder::build_pipeline(
                                        &cfg,
                                        enc.as_ref(),
                                        &platform_ctx,
                                    );

                                    info!("restarting pipeline for monitor switch: {}", pipeline_str);
                                    if let Err(e) = stream.restart_pipeline(&pipeline_str) {
                                        error!("monitor switch restart failed: {}. Falling back to monitor 0.", e);
                                        if cfg.monitor_index != 0 {
                                            cfg.monitor_index = 0;
                                            config.set(cfg.clone());
                                            let fallback_pipe = pipeline::PipelineBuilder::build_pipeline(&cfg, enc.as_ref(), &platform_ctx);
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
                                #[cfg(target_os = "windows")]
                                input::handle_mouse_windows(input);
                                #[cfg(target_os = "linux")]
                                tracing::debug!("Linux mouse input routed through PCLink Core /dev/uinput: {:?}", input);
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

    #[cfg(target_os = "linux")]
    let _keep_portal = portal_capture_arc;

    tokio::signal::ctrl_c().await?;
    info!("shutting down");
    #[cfg(target_os = "linux")]
    drop(_keep_portal);
    Ok(())
}