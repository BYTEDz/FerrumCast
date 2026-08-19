// src/platform/windows/mod.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

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
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION};

use crate::config::StreamConfig;
use crate::input::MouseInput;
use crate::loc;
use crate::platform::{DisplayServer, PlatformBackend, VideoSourceDescriptor};

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

    fn is_remote_session() -> bool {
        unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn is_hypervisor() -> bool {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::__cpuid;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::__cpuid;

        let res = unsafe { __cpuid(1) };
        (res.ecx & (1 << 31)) != 0
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    fn is_hypervisor() -> bool {
        false
    }
}

impl PlatformBackend for WindowsBackend {
    fn detect_display_server(&self) -> DisplayServer {
        if Self::is_remote_session() || Self::is_hypervisor() {
            DisplayServer::WindowsHeadlessOrVm
        } else {
            DisplayServer::WindowsDesktop
        }
    }

    fn build_video_source(
        &self,
        cfg: &StreamConfig,
        is_hardware_encoder: bool,
    ) -> VideoSourceDescriptor {
        let is_vm = Self::is_hypervisor() || Self::is_remote_session();
        let use_gdi = cfg.gdi || (is_vm && !is_hardware_encoder);

        if use_gdi {
            VideoSourceDescriptor {
                pipeline_fragment: "appsrc name=gdi_src format=time is-live=true do-timestamp=true block=false max-bytes=20000000 ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream".to_string(),
                preferred_memory_feature: None,
                preferred_converter: "videoconvert n-threads=0".to_string(),
                raw_caps_filter: None,
            }
        } else if is_hardware_encoder {
            VideoSourceDescriptor {
                pipeline_fragment: format!(
                    "d3d11screencapturesrc show-cursor={} monitor-index={} ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream",
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
                    "d3d11screencapturesrc show-cursor={} monitor-index={} ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream",
                    if cfg.show_cursor { "true" } else { "false" },
                    cfg.monitor_index
                ),
                preferred_memory_feature: None,
                preferred_converter: "d3d11convert ! d3d11download ! videoconvert n-threads=0"
                    .to_string(),
                raw_caps_filter: Some(format!("video/x-raw,format={} ! ", target_format)),
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
            *self.gdi_capture_handle.lock() = Some(gdi_capture::start_gdi_capture(appsrc));
        }
        Ok(())
    }

    fn pre_pipeline_stop(&self) {
        if let Some(handle) = self.gdi_capture_handle.lock().take() {
            handle.store(false, Ordering::SeqCst);
        }
    }
}
