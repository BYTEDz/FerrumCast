// src/platform/linux/display.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use std::env;
use std::path::PathBuf;
use tracing::info;

use crate::loc;
use crate::platform::DisplayServer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxDisplayEnvironment {
    Wayland,
    X11,
    Headless,
}

impl LinuxDisplayEnvironment {
    pub fn probe() -> Self {
        if Self::is_wayland_active() {
            info!("{}", loc::MSG_ENV_DETECTED_WAYLAND);
            LinuxDisplayEnvironment::Wayland
        } else if Self::is_x11_active() {
            info!("{}", loc::MSG_ENV_DETECTED_X11);
            LinuxDisplayEnvironment::X11
        } else {
            info!("{}", loc::MSG_ENV_DETECTED_HEADLESS);
            LinuxDisplayEnvironment::Headless
        }
    }

    fn is_wayland_active() -> bool {
        // If explicitly set to X11, do not treat as Wayland even if stale sockets exist
        if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
            if session_type.trim().eq_ignore_ascii_case("x11") {
                return false;
            }
            if session_type.trim().eq_ignore_ascii_case("wayland") {
                return true;
            }
        }

        // Check active WAYLAND_DISPLAY variable
        if let Ok(wayland_display) = env::var("WAYLAND_DISPLAY") {
            let trimmed = wayland_display.trim();
            if !trimmed.is_empty() {
                if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
                    return PathBuf::from(runtime_dir).join(trimmed).exists();
                }
                return true;
            }
        }

        false
    }

    fn is_x11_active() -> bool {
        if let Ok(val) = env::var("DISPLAY") {
            !val.trim().is_empty()
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn to_display_server(self) -> DisplayServer {
        match self {
            LinuxDisplayEnvironment::Wayland => DisplayServer::Wayland,
            LinuxDisplayEnvironment::X11 => DisplayServer::X11,
            LinuxDisplayEnvironment::Headless => DisplayServer::Unknown,
        }
    }
}
