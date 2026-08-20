// src/platform/linux/mod.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

pub mod display;
pub mod portal;

use anyhow::Result;
use gstreamer as gst;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tracing::{error, info, warn};

use crate::config::StreamConfig;
use crate::input::MouseInput;
use crate::ipc::OutboundMessage;
use crate::loc;
use crate::pipeline::encoders::VideoEncoder;
use crate::platform::{DisplayServer, PlatformBackend, VideoSourceDescriptor};
use display::LinuxDisplayEnvironment;

pub struct LinuxBackend {
    display_env: LinuxDisplayEnvironment,
    pub portal_capture: Option<Arc<portal::PortalCapture>>,
}

impl LinuxBackend {
    pub async fn new(
        initial_token: Option<String>,
        outbound_tx: Sender<OutboundMessage>,
        audio_only: bool,
    ) -> Result<Self> {
        let display_env = LinuxDisplayEnvironment::probe();

        let portal_capture = if display_env == LinuxDisplayEnvironment::Wayland && !audio_only {
            let token_path = Self::resolve_token_file_path();
            let effective_token = initial_token.or_else(|| {
                std::fs::read_to_string(&token_path)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            });

            let had_token = effective_token.is_some();
            let capture_result =
                portal::request_screencast(effective_token, Some(outbound_tx.clone())).await;

            let capture_result = match capture_result {
                Ok(c) => Ok(c),
                Err(ref e) if had_token => {
                    warn!(
                        "{}: {}. Purging cached token and requesting fresh permission.",
                        loc::MSG_SAVED_TOKEN_INVALID,
                        e
                    );
                    let _ = std::fs::remove_file(&token_path);
                    portal::request_screencast(None, Some(outbound_tx.clone())).await
                }
                Err(e) => Err(e),
            };

            match capture_result {
                Ok(c) => {
                    if let Some(ref t) = c.restore_token {
                        info!("{}: {:?}", loc::MSG_SAVED_TOKEN_PERSISTING, token_path);
                        if let Some(parent) = token_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&token_path, t);
                        let _ = outbound_tx
                            .send(OutboundMessage::PortalTokenGenerated { token: t.clone() });
                        println!("PORTAL_TOKEN_SAVED: {}", t);
                    }
                    Some(Arc::new(c))
                }
                Err(e) => {
                    error!("{}: {}", loc::MSG_PORTAL_FAILED, e);
                    None
                }
            }
        } else {
            None
        };

        info!("{}", loc::MSG_BACKEND_INITIALIZED);
        Ok(Self {
            display_env,
            portal_capture,
        })
    }

    pub fn resolve_token_file_path() -> std::path::PathBuf {
        if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(config_home).join("ferrumcast.token")
        } else if let Ok(home) = std::env::var("HOME") {
            std::path::PathBuf::from(home)
                .join(".config")
                .join("ferrumcast.token")
        } else {
            std::path::PathBuf::from("/tmp/ferrumcast.token")
        }
    }
}

impl PlatformBackend for LinuxBackend {
    fn detect_display_server(&self) -> DisplayServer {
        self.display_env.to_display_server()
    }

    fn build_video_source(
        &self,
        cfg: &StreamConfig,
        encoder: &dyn VideoEncoder,
    ) -> VideoSourceDescriptor {
        let is_nvenc = encoder.is_nvenc();

        let (preferred_converter, preferred_memory_feature) = if is_nvenc {
            (
                "glupload ! glcolorconvert".to_string(),
                Some("video/x-raw(memory:GLMemory)"),
            )
        } else {
            ("videoconvert n-threads=0".to_string(), None)
        };

        let queue_cap = if encoder.is_zero_latency_capable() {
            "queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream"
        } else {
            "queue max-size-buffers=3 max-size-bytes=0 max-size-time=33000000"
        };

        let pipeline_fragment = if let Some(ref portal) = self.portal_capture {
            format!(
                "pipewiresrc fd={} path={} do-timestamp=true always-copy=false ! {}",
                portal.fd, portal.node_id, queue_cap
            )
        } else if self.display_env == LinuxDisplayEnvironment::Wayland {
            format!("videotestsrc is-live=true ! {}", queue_cap)
        } else {
            format!(
                "ximagesrc use-damage=true show-pointer={} do-timestamp=true ! {}",
                if cfg.show_cursor { "true" } else { "false" },
                queue_cap
            )
        };

        VideoSourceDescriptor {
            pipeline_fragment,
            preferred_memory_feature,
            preferred_converter,
            raw_caps_filter: None,
        }
    }

    fn build_audio_source(&self, _cfg: &StreamConfig) -> String {
        "pulsesrc buffer-time=10000 latency-time=10000".to_string()
    }

    fn handle_mouse_input(&self, input: &MouseInput) {
        tracing::debug!(
            "Linux mouse input received for kernel uinput dispatcher: {:?}",
            input
        );
    }

    fn post_pipeline_start(&self, _pipeline: &gst::Pipeline) -> Result<()> {
        Ok(())
    }

    fn pre_pipeline_stop(&self) {}
}
