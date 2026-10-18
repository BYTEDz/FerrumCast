// src/pipeline/encoders/software.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use super::VideoEncoder;
use crate::config::StreamConfig;

pub struct X264Encoder;
impl VideoEncoder for X264Encoder {
    fn gst_element(&self) -> &'static str {
        "x264enc"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";

        if is_cqp {
            format!(
                "quantizer={cqp} tune={tune} speed-preset={preset} \
                rc-lookahead=0 sync-lookahead=0 key-int-max={key_int} bframes=0 \
                threads=0 sliced-threads=true b-adapt=false \
                option-string=repeat-headers=1",
                cqp = cfg.cqp_value,
                tune = cfg.tune,
                preset = cfg.speed_preset,
                key_int = key_int,
            )
        } else {
            let vbv = ((bitrate as f32 * 0.05) as u32).max(100);
            format!(
                "bitrate={bitrate} tune={tune} speed-preset={preset} \
                rc-lookahead=0 sync-lookahead=0 key-int-max={key_int} bframes=0 \
                threads=0 sliced-threads=true b-adapt=false \
                option-string=nal-hrd=cbr:repeat-headers=1:vbv-maxrate={bitrate}:vbv-bufsize={vbv}",
                bitrate = bitrate,
                tune = cfg.tune,
                preset = cfg.speed_preset,
                key_int = key_int,
                vbv = vbv,
            )
        }
    }

    fn pre_caps(&self) -> Option<&'static str> {
        None
    }

    fn is_hardware(&self) -> bool {
        false
    }
}

pub struct X265Encoder;
impl VideoEncoder for X265Encoder {
    fn gst_element(&self) -> &'static str {
        "x265enc"
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
                "speed-preset={preset} tune=zerolatency key-int-max={key_int} \
                option-string=qp={cqp}:bframes=0:repeat-headers=1:no-open-gop=1:rc-lookahead=0",
                preset = cfg.speed_preset,
                key_int = key_int,
                cqp = cfg.cqp_value,
            )
        } else {
            format!(
                "bitrate={bitrate} speed-preset={preset} tune=zerolatency key-int-max={key_int} \
                option-string=bframes=0:repeat-headers=1:no-open-gop=1:rc-lookahead=0",
                bitrate = bitrate,
                preset = cfg.speed_preset,
                key_int = key_int,
            )
        }
    }

    fn pre_caps(&self) -> Option<&'static str> {
        None
    }

    fn is_hardware(&self) -> bool {
        false
    }

    fn is_zero_latency_capable(&self) -> bool {
        false
    }
}