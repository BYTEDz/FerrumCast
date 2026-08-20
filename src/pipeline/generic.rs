// src/pipeline/generic.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use crate::config::StreamConfig;

pub fn scale_caps(
    cfg: &StreamConfig,
    format: Option<&str>,
    is_hw: bool,
    mem_feature: Option<&str>,
) -> String {
    let mut pre_elements = String::new();
    let mut parts = Vec::new();

    let has_target_res = cfg.width.map_or(false, |w| w > 0) || cfg.height.map_or(false, |h| h > 0);

    if has_target_res && mem_feature.is_none() {
        pre_elements.push_str("videoscale ! ");
    }

    if cfg.framerate.map_or(false, |f| f > 0) && mem_feature.is_none() {
        pre_elements.push_str("videorate ! ");
    }

    if let Some(fps) = cfg.framerate {
        if fps > 0 {
            parts.push(format!("framerate={}/1", fps));
        }
    }

    if let Some(w) = cfg.width {
        if w > 0 {
            parts.push(format!("width={}", w));
        }
    }

    if let Some(h) = cfg.height {
        if h > 0 {
            parts.push(format!("height={}", h));
        }
    }

    let format_str = if is_hw {
        format.unwrap_or("NV12")
    } else {
        format.unwrap_or("I420")
    };

    parts.push(format!("format={}", format_str));

    let media_type = mem_feature.unwrap_or("video/x-raw");
    let caps_string = format!("{},{} ! ", media_type, parts.join(","));

    format!("{}{}", pre_elements, caps_string)
}
