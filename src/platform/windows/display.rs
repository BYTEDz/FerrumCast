// src/platform/windows/display.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

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
    pub fn probe(force_gdi: bool, is_hardware_encoder: bool) -> Self {
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

        if Self::is_hypervisor() && !is_hardware_encoder {
            info!(
                "{}: hypervisor without hardware encoder",
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

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn is_hypervisor() -> bool {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::__cpuid;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::__cpuid;

        let res = unsafe { __cpuid(1) };
        (res.ecx & (1 << 31)) != 0
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    pub fn is_hypervisor() -> bool {
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
