// src/platform/linux/mod.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

pub mod portal;

use anyhow::Result;
use gstreamer as gst;
use std::env;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tracing::{error, info};

use crate::config::StreamConfig;
use crate::input::MouseInput;
use crate::ipc::OutboundMessage;
use crate::loc;
use crate::platform::{DisplayServer, PlatformBackend, VideoSourceDescriptor};

pub struct LinuxBackend {
    pub portal_capture: Option<Arc<portal::PortalCapture>>,
}

impl LinuxBackend {
    pub async fn new(
        initial_token: Option<String>,
        outbound_tx: Sender<OutboundMessage>,
        audio_only: bool,
    ) -> Result<Self> {
        let is_wayland = env::var("WAYLAND_DISPLAY").is_ok()
            || env::var("XDG_SESSION_TYPE")
                .map(|v| v.to_lowercase() == "wayland")
                .unwrap_or(false);

        let portal_capture = if is_wayland && !audio_only {
            let token_path = Self::resolve_token_file_path();
            let capture_result =
                portal::request_screencast(initial_token.clone(), Some(outbound_tx.clone())).await;

            let capture_result = match capture_result {
                Ok(c) => Ok(c),
                Err(ref e) if initial_token.is_some() => {
                    error!(
                        "{}: {}. Purging cached token.",
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
        Ok(Self { portal_capture })
    }

    pub fn resolve_token_file_path() -> std::path::PathBuf {
        if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(config_home).join("ferrumcast.token")
        } else if let Ok(home) = env::var("HOME") {
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
        if env::var("WAYLAND_DISPLAY").is_ok()
            || env::var("XDG_SESSION_TYPE")
                .map(|v| v.to_lowercase() == "wayland")
                .unwrap_or(false)
        {
            DisplayServer::Wayland
        } else if env::var("DISPLAY").is_ok() {
            DisplayServer::X11
        } else {
            DisplayServer::Unknown
        }
    }

    fn build_video_source(
        &self,
        cfg: &StreamConfig,
        _is_hardware_encoder: bool,
    ) -> VideoSourceDescriptor {
        let is_vaapi = matches!(
            cfg.encoder,
            crate::config::EncoderChoice::VaH264 | crate::config::EncoderChoice::VaH265
        );
        let is_nvenc = matches!(
            cfg.encoder,
            crate::config::EncoderChoice::Nvenc | crate::config::EncoderChoice::NvencH265
        );

        let preferred_memory_feature = if is_vaapi {
            Some("video/x-raw(memory:VAMemory)")
        } else if is_nvenc {
            Some("video/x-raw(memory:GLMemory)")
        } else {
            None
        };

        let preferred_converter = if is_vaapi {
            "vapostproc".to_string()
        } else if is_nvenc {
            "glcolorconvert".to_string()
        } else {
            "videoconvert n-threads=0".to_string()
        };

        let pipeline_fragment = if let Some(ref portal) = self.portal_capture {
            format!(
                "pipewiresrc fd={} path={} do-timestamp=true always-copy=false ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream",
                portal.fd, portal.node_id
            )
        } else if self.detect_display_server() == DisplayServer::Wayland {
            "videotestsrc is-live=true ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream"
                .to_string()
        } else {
            format!(
                "ximagesrc use-damage=true show-pointer={} do-timestamp=true ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream",
                if cfg.show_cursor { "true" } else { "false" }
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
