// src/pipeline/encoders/vaapi.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use super::VideoEncoder;
use crate::config::StreamConfig;

pub struct VaH264Encoder;
impl VideoEncoder for VaH264Encoder {
    fn gst_element(&self) -> &'static str {
        "vah264enc"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";

        if is_cqp {
            format!(
                "rate-control=cqp qpi={cqp} qpp={cqp} key-int-max={key_int} \
                aud=false ref-frames=1 b-frames=0",
                cqp = cfg.cqp_value,
                key_int = key_int,
            )
        } else {
            let rc = match cfg.rc_mode.as_str() {
                "vbr" => "vbr",
                _ => "cbr",
            };
            format!(
                "bitrate={bitrate} rate-control={rc} key-int-max={key_int} \
                aud=false ref-frames=1 b-frames=0",
                bitrate = bitrate,
                rc = rc,
                key_int = key_int,
            )
        }
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_vaapi_native(&self) -> bool {
        true
    }

    fn is_zero_latency_capable(&self) -> bool {
        true
    }
}

pub struct VaH265Encoder;
impl VideoEncoder for VaH265Encoder {
    fn gst_element(&self) -> &'static str {
        "vah265enc"
    }

    fn codec_name(&self) -> &'static str {
        "h265"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";

        if is_cqp {
            format!(
                "rate-control=cqp qpi={cqp} qpp={cqp} key-int-max={key_int} \
                aud=false ref-frames=1 b-frames=0",
                cqp = cfg.cqp_value,
                key_int = key_int,
            )
        } else {
            let rc = match cfg.rc_mode.as_str() {
                "vbr" => "vbr",
                _ => "cbr",
            };
            format!(
                "bitrate={bitrate} rate-control={rc} key-int-max={key_int} \
                aud=false ref-frames=1 b-frames=0",
                bitrate = bitrate,
                rc = rc,
                key_int = key_int,
            )
        }
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_vaapi_native(&self) -> bool {
        true
    }

    fn is_zero_latency_capable(&self) -> bool {
        true
    }
}