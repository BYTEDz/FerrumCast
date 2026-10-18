// src/pipeline/encoders/nvenc.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use super::VideoEncoder;
use crate::config::StreamConfig;

pub struct NvencEncoder;
impl VideoEncoder for NvencEncoder {
    fn gst_element(&self) -> &'static str {
        "nvh264enc"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";

        let rc = if is_cqp {
            "constqp"
        } else if cfg.rc_mode == "vbr" {
            "vbr"
        } else {
            "cbr-ld-hq"
        };

        if is_cqp {
            format!(
                "qp-const-i={cqp} zerolatency=true rc-lookahead=0 spatial-aq=false temporal-aq=false \
                preset={preset} tune={tune} rc={rc} key-int-max={key_int} b-frames=0 ref=1",
                cqp = cfg.cqp_value,
                preset = cfg.nvenc_preset,
                tune = cfg.nvenc_tune,
                rc = rc,
                key_int = key_int,
            )
        } else {
            format!(
                "bitrate={bitrate} zerolatency=true rc-lookahead=0 spatial-aq=false temporal-aq=false \
                preset={preset} tune={tune} rc={rc} key-int-max={key_int} b-frames=0 ref=1",
                bitrate = bitrate,
                preset = cfg.nvenc_preset,
                tune = cfg.nvenc_tune,
                rc = rc,
                key_int = key_int,
            )
        }
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_nvenc(&self) -> bool {
        true
    }
}

pub struct NvencH265Encoder;
impl VideoEncoder for NvencH265Encoder {
    fn gst_element(&self) -> &'static str {
        "nvh265enc"
    }

    fn codec_name(&self) -> &'static str {
        "h265"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";

        let rc = if is_cqp {
            "constqp"
        } else if cfg.rc_mode == "vbr" {
            "vbr"
        } else {
            "cbr-ld-hq"
        };

        if is_cqp {
            format!(
                "qp-const-i={cqp} zerolatency=true rc-lookahead=0 spatial-aq=false temporal-aq=false \
                preset={preset} tune={tune} rc={rc} key-int-max={key_int} b-frames=0 ref=1",
                cqp = cfg.cqp_value,
                preset = cfg.nvenc_preset,
                tune = cfg.nvenc_tune,
                rc = rc,
                key_int = key_int,
            )
        } else {
            format!(
                "bitrate={bitrate} zerolatency=true rc-lookahead=0 spatial-aq=false temporal-aq=false \
                preset={preset} tune={tune} rc={rc} key-int-max={key_int} b-frames=0 ref=1",
                bitrate = bitrate,
                preset = cfg.nvenc_preset,
                tune = cfg.nvenc_tune,
                rc = rc,
                key_int = key_int,
            )
        }
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_nvenc(&self) -> bool {
        true
    }
}