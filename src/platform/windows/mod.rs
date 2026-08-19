// src/platform/windows/mod.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

pub mod display;
pub mod gdi_capture;
pub mod input;

use anyhow::Result;
use gstreamer as gst;
use gstreamer_app as gst_app;
use parking_lot::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tracing::info;

use crate::config::StreamConfig;
use crate::input::MouseInput;
use crate::loc;
use crate::platform::{DisplayServer, PlatformBackend, VideoSourceDescriptor};
use display::WindowsDisplayEnvironment;

pub struct WindowsBackend {
    gdi_capture_handle: Mutex<Option<Arc<AtomicBool>>>,
}

impl WindowsBackend {
    pub fn new() -> Self {
        info!("{}", loc::MSG_BACKEND_INITIALIZED);
        Self {
            gdi_capture_handle: Mutex::new(None),
        }
    }
}

impl PlatformBackend for WindowsBackend {
    fn detect_display_server(&self) -> DisplayServer {
        WindowsDisplayEnvironment::probe(false, false).to_display_server()
    }

    fn build_video_source(
        &self,
        cfg: &StreamConfig,
        is_hardware_encoder: bool,
    ) -> VideoSourceDescriptor {
        let env = WindowsDisplayEnvironment::probe(cfg.gdi, is_hardware_encoder);

        match env {
            WindowsDisplayEnvironment::GdiRequired => VideoSourceDescriptor {
                pipeline_fragment: "appsrc name=gdi_src format=time is-live=true do-timestamp=true block=false max-bytes=20000000 ! queue max-size-buffers=3 max-size-bytes=0 max-size-time=33000000 leaky=downstream".to_string(),
                preferred_memory_feature: None,
                preferred_converter: "videoconvert n-threads=0".to_string(),
                raw_caps_filter: None,
            },
            WindowsDisplayEnvironment::Direct3D11 => {
                if is_hardware_encoder {
                    VideoSourceDescriptor {
                        pipeline_fragment: format!(
                            "d3d11screencapturesrc show-cursor={} monitor-index={} ! queue max-size-buffers=3 max-size-bytes=0 max-size-time=33000000 leaky=downstream",
                            if cfg.show_cursor { "true" } else { "false" },
                            cfg.monitor_index
                        ),
                        preferred_memory_feature: Some("video/x-raw(memory:D3D11Memory)"),
                        preferred_converter: "d3d11convert".to_string(),
                        raw_caps_filter: None,
                    }
                } else {
                    let target_format = "NV12";
                    VideoSourceDescriptor {
                        pipeline_fragment: format!(
                            "d3d11screencapturesrc show-cursor={} monitor-index={} ! queue max-size-buffers=3 max-size-bytes=0 max-size-time=33000000 leaky=downstream",
                            if cfg.show_cursor { "true" } else { "false" },
                            cfg.monitor_index
                        ),
                        preferred_memory_feature: None,
                        preferred_converter: "d3d11convert ! d3d11download ! videoconvert n-threads=0".to_string(),
                        raw_caps_filter: Some(format!("video/x-raw,format={} ! ", target_format)),
                    }
                }
            }
        }
    }

    fn build_audio_source(&self, _cfg: &StreamConfig) -> String {
        "wasapisrc loopback=true low-latency=true do-timestamp=true".to_string()
    }

    fn handle_mouse_input(&self, input: &MouseInput) {
        input::handle_mouse_windows(input);
    }

    fn post_pipeline_start(&self, pipeline: &gst::Pipeline) -> Result<()> {
        if let Some(src) = pipeline.by_name("gdi_src") {
            let appsrc = src.downcast::<gst_app::AppSrc>().unwrap();
            *self.gdi_capture_handle.lock() = Some(gdi_capture::start_gdi_capture(appsrc, 60));
        }
        Ok(())
    }

    fn pre_pipeline_stop(&self) {
        if let Some(handle) = self.gdi_capture_handle.lock().take() {
            handle.store(false, Ordering::SeqCst);
        }
    }
}
