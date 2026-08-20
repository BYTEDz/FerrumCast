// src/platform/windows/display.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use gstreamer as gst;
use gstreamer::prelude::*;
use tracing::info;
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION};

use crate::loc;
use crate::platform::DisplayServer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsDisplayEnvironment {
    Direct3D11,
    GdiRequired,
}

impl WindowsDisplayEnvironment {
    pub fn probe(force_gdi: bool, _is_hardware_encoder: bool) -> Self {
        if force_gdi {
            info!("{}: explicit cli flag", loc::MSG_ENV_DETECTED_GDI_FALLBACK);
            return WindowsDisplayEnvironment::GdiRequired;
        }

        if Self::is_remote_desktop_session() {
            info!(
                "{}: rdp/session 0 active",
                loc::MSG_ENV_DETECTED_GDI_FALLBACK
            );
            return WindowsDisplayEnvironment::GdiRequired;
        }

        if !Self::is_d3d11_capture_available() {
            info!(
                "{}: D3D11 desktop duplication unsupported in this environment (VM fallback)",
                loc::MSG_ENV_DETECTED_GDI_FALLBACK
            );
            return WindowsDisplayEnvironment::GdiRequired;
        }

        info!("{}", loc::MSG_ENV_DETECTED_D3D11);
        WindowsDisplayEnvironment::Direct3D11
    }

    pub fn is_remote_desktop_session() -> bool {
        unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 }
    }

    fn is_d3d11_capture_available() -> bool {
        let test_pipe = "d3d11screencapturesrc num-buffers=1 ! fakesink sync=false";
        if let Ok(elem) = gst::parse::launch(test_pipe) {
            if let Ok(pipeline) = elem.dynamic_cast::<gst::Pipeline>() {
                let res = pipeline.set_state(gst::State::Paused);
                let is_ok = match res {
                    Ok(gst::StateChangeSuccess::Success) => true,
                    Ok(gst::StateChangeSuccess::Async) => {
                        let (state_res, _, _) =
                            pipeline.state(Some(gst::ClockTime::from_mseconds(300)));
                        state_res.is_ok()
                    }
                    _ => false,
                };
                let _ = pipeline.set_state(gst::State::Null);
                return is_ok;
            }
        }
        false
    }

    #[allow(dead_code)]
    pub fn to_display_server(self) -> DisplayServer {
        match self {
            WindowsDisplayEnvironment::Direct3D11 => DisplayServer::WindowsDesktop,
            WindowsDisplayEnvironment::GdiRequired => DisplayServer::WindowsHeadlessOrVm,
        }
    }
}