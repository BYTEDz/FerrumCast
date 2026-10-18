// src/platform/quirks.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuVendor {
    Amd,
    Intel,
    Nvidia,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuQuirks {
    pub vendor: GpuVendor,
    pub device_id: u32,
    pub broken_vce_encode: bool,
    pub broken_hw_hevc: bool,
    pub prefer_software_fallback: bool,
}

impl Default for GpuQuirks {
    fn default() -> Self {
        Self {
            vendor: GpuVendor::Unknown,
            device_id: 0,
            broken_vce_encode: false,
            broken_hw_hevc: false,
            prefer_software_fallback: false,
        }
    }
}

impl GpuQuirks {
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            Self::detect_linux()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Self::default()
        }
    }

    #[cfg(target_os = "linux")]
    fn detect_linux() -> Self {
        let allow_unstable = std::env::var("FERRUMCAST_ALLOW_UNSTABLE_VCE")
            .map_or(false, |v| v == "1" || v.eq_ignore_ascii_case("true"))
            || std::env::args().any(|arg| arg == "--allow-unstable-vce");

        let mut quirks = Self::default();

        let drm_dir = std::path::Path::new("/sys/class/drm");
        if let Ok(entries) = std::fs::read_dir(drm_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                // Inspect primary GPU cards (e.g., card0, card1), skip render nodes and virtual displays
                if !name_str.starts_with("card") || name_str.contains('-') {
                    continue;
                }

                let dev_path = entry.path().join("device");
                let vendor_path = dev_path.join("vendor");
                let device_path = dev_path.join("device");

                let vendor_id = std::fs::read_to_string(vendor_path)
                    .ok()
                    .and_then(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok());

                let dev_id = std::fs::read_to_string(device_path)
                    .ok()
                    .and_then(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok());

                if let (Some(vendor), Some(device)) = (vendor_id, dev_id) {
                    match vendor {
                        0x1002 => {
                            quirks.vendor = GpuVendor::Amd;
                            quirks.device_id = device;

                            // Polaris / Ellesmere / Baffin family (VCE 3.0 / 3.4)
                            // PCI ID ranges: 0x67C0..=0x67FF, 0x6FDF (RX 580 2048SP)
                            let is_polaris = (0x67C0..=0x67FF).contains(&device) || device == 0x6FDF;
                            if is_polaris && !allow_unstable {
                                quirks.broken_vce_encode = true;
                                quirks.broken_hw_hevc = true;
                                quirks.prefer_software_fallback = true;
                            }
                        }
                        0x8086 => {
                            quirks.vendor = GpuVendor::Intel;
                            quirks.device_id = device;
                        }
                        0x10DE => {
                            quirks.vendor = GpuVendor::Nvidia;
                            quirks.device_id = device;
                        }
                        _ => {}
                    }
                    break;
                }
            }
        }

        quirks
    }

    pub fn is_encoder_unstable(&self, encoder_label: &str) -> bool {
        match encoder_label {
            "vah264" => self.broken_vce_encode,
            "vah265" => self.broken_hw_hevc,
            _ => false,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Vendor: {:?}, DeviceId: 0x{:04X}, VCE-Broken: {}, HEVC-Broken: {}",
            self.vendor, self.device_id, self.broken_vce_encode, self.broken_hw_hevc
        )
    }
}