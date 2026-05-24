mod config;
#[cfg(target_os = "windows")]
mod gdi_capture;
mod ipc;
mod pipeline;
mod webrtc;

#[cfg(target_os = "linux")]
mod portal;

use anyhow::Result;
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

use std::sync::Arc;

#[cfg(target_os = "linux")]
const TOKEN_FILE: &str = "/tmp/ferrumcast.token";

#[tokio::main]
async fn main() -> Result<()> {
    // Enable system-level DPI awareness on Windows to prevent coordinate scaling or virtualization issues.
    // Without this call, Windows scales down system metrics and screen sizes on high-DPI displays,
    // which results in capturing only the upper-left portion of the window or screen.
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

    info!("starting ferrumcast engine");

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--probe") {
        gstreamer::init().expect("Failed to initialize gstreamer");
        let caps = pipeline::PipelineBuilder::probe_capabilities();
        println!("{}", serde_json::to_string(&caps).unwrap());
        std::process::exit(0);
    }

    let (outbound_tx, _outbound_rx) = tokio::sync::broadcast::channel(32);

    // Configure environment variables for portable, self-contained GStreamer runtimes on Windows,
    // allowing the engine to locate localized binaries and plugins without a system-wide installation.
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

                // Prepend the executable directory to the PATH environment variable
                // to facilitate dynamic link library (DLL) resolution for localized GStreamer dependencies.
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

    // GStreamer global state initialization must precede any GObject inspections,
    // registry queries, or pipeline parsing operations.
    gstreamer::init().expect("Failed to initialize gstreamer");

    let caps = Arc::new(pipeline::PipelineBuilder::probe_capabilities());
    info!("available encoders: {:?}", caps.encoders);

    // Initialize the configuration registry, pre-seeded with CLI parameters from the orchestrator layer.
    let config_store = Arc::new(config::ConfigStore::new_from_args());

    // Retrieve the screen-capture token, falling back to local persistent storage if not provided in args.
    #[cfg(target_os = "linux")]
    let initial_token = {
        let cfg = config_store.get();
        if cfg.token.is_some() {
            cfg.token
        } else {
            std::fs::read_to_string(TOKEN_FILE).ok()
        }
    };

    #[cfg(target_os = "linux")]
    let ipc_path = "/tmp/ferrumcast.sock";
    #[cfg(target_os = "windows")]
    let ipc_path = r"\\.\pipe\ferrumcast";

    // Unlink the legacy Unix domain socket path to prevent binding failures (EADDRINUSE).
    #[cfg(target_os = "linux")]
    let _ = std::fs::remove_file(ipc_path);

    info!("binding IPC to {}", ipc_path);
    let server = Arc::new(ipc::IpcServer::new(ipc_path));

    // Under Wayland, negotiate screen-cast authorization with the XDG Desktop Portal to
    // obtain the required file descriptor and PipeWire node identifier.
    #[cfg(target_os = "linux")]
    let portal_capture = if pipeline::PipelineBuilder::is_wayland() {
        match portal::request_screencast(initial_token, Some(outbound_tx.clone())).await {
            Ok(c) => {
                // Cache the restore token to allow silent session resumption in future instances.
                if let Some(ref t) = c.restore_token {
                    info!("persisting portal token to {}", TOKEN_FILE);
                    let _ = std::fs::write(TOKEN_FILE, t);
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

    let platform_ctx = Arc::new(pipeline::PlatformContext {
        #[cfg(target_os = "linux")]
        portal_info: portal_capture.as_ref().map(|c| (c.node_id, c.fd)),
    });

    let initial_cfg = config_store.get();
    let enc = config::resolve_encoder(&initial_cfg.encoder, &caps);
    let pipeline_str = pipeline::PipelineBuilder::build_pipeline(&initial_cfg, &enc, &platform_ctx);

    info!("pipeline: {}", pipeline_str);

    let stream_manager = match webrtc::StreamManager::new(&pipeline_str, outbound_tx.clone()) {
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
                            ipc::InboundMessage::SetRemoteSdp { sdp, sdp_type } => {
                                let _ = stream.handle_remote_sdp(&sdp, &sdp_type);
                            }
                            ipc::InboundMessage::AddIceCandidate {
                                candidate,
                                mid,
                                mline_index,
                            } => {
                                let _ = stream.add_ice_candidate(&candidate, mid, mline_index);
                            }
                            ipc::InboundMessage::RequestOffer => {
                                let _ = stream.generate_offer();
                            }
                            ipc::InboundMessage::StopStream => {
                                info!("stopping pipeline (engine stays alive)");
                                let _ = stream.stop();
                            }
                            ipc::InboundMessage::RestartPipeline(cfg) => {
                                info!(
                                    "restarting pipeline via IPC: host={} encoder={:?}",
                                    cfg.client_host, cfg.encoder
                                );
                                config.set(cfg.clone());

                                let enc = config::resolve_encoder(&cfg.encoder, &caps);
                                let pipeline_str = pipeline::PipelineBuilder::build_pipeline(
                                    &cfg,
                                    &enc,
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
                            ipc::InboundMessage::ConfigureStream(cfg) => {
                                info!("stream config updated: bitrate={}kbps", cfg.bitrate);
                                config.set(cfg.clone());
                                if let Err(e) = stream.update_bitrate(cfg.bitrate) {
                                    error!("Failed to update bitrate dynamically: {}", e);
                                }
                                let _ = tx.send(ipc::OutboundMessage::ConfigAck {
                                    active_encoder: stream.active_encoder(),
                                });
                            }
                            ipc::InboundMessage::GetCapabilities => {
                                let _ = tx.send(ipc::OutboundMessage::CapabilitiesResponse(
                                    (*caps).clone(),
                                ));
                            }
                            ipc::InboundMessage::ForceKeyframe => {
                                let _ = stream.force_keyframe();
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

    // Explicitly bind the lifetime of the portal capture context to the application lifetime
    // to prevent RAII cleanup from dropping the D-Bus session and terminating the stream.
    #[cfg(target_os = "linux")]
    let _keep_portal = portal_capture;

    tokio::signal::ctrl_c().await?;
    info!("shutting down");
    #[cfg(target_os = "linux")]
    drop(_keep_portal);
    Ok(())
}