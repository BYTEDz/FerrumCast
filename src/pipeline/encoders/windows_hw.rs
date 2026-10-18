// src/pipeline/encoders/windows_hw.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use super::VideoEncoder;
use crate::config::StreamConfig;

pub struct D3d11H264Encoder;
impl VideoEncoder for D3d11H264Encoder {
    fn gst_element(&self) -> &'static str {
        "d3d11h264enc"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";
        let rc = if is_cqp { "cqp" } else { cfg.rc_mode.as_str() };

        format!(
            "bitrate={bitrate} rate-control={rc} gop-size={key_int} b-frames=0 \
            low-latency=true async-depth=1",
            bitrate = bitrate,
            rc = rc,
            key_int = key_int,
        )
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_d3d11_native(&self) -> bool {
        true
    }
}

pub struct D3d11H265Encoder;
impl VideoEncoder for D3d11H265Encoder {
    fn gst_element(&self) -> &'static str {
        "d3d11h265enc"
    }

    fn codec_name(&self) -> &'static str {
        "h265"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";
        let rc = if is_cqp { "cqp" } else { cfg.rc_mode.as_str() };

        format!(
            "bitrate={bitrate} rate-control={rc} gop-size={key_int} b-frames=0 \
            low-latency=true async-depth=1",
            bitrate = bitrate,
            rc = rc,
            key_int = key_int,
        )
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_d3d11_native(&self) -> bool {
        true
    }
}

pub struct QsvEncoder;
impl VideoEncoder for QsvEncoder {
    fn gst_element(&self) -> &'static str {
        "qsvh264enc"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";
        let rc = if is_cqp { "cqp" } else { cfg.rc_mode.as_str() };

        if is_cqp {
            format!(
                "qpi={cqp} qpp={cqp} qpb={cqp} target-usage={tu} rate-control={rc} gop-size={key_int} \
                b-frames=0 ref-frames=1 low-latency=true async-depth=1",
                cqp = cfg.cqp_value,
                tu = cfg.qsv_target_usage,
                rc = rc,
                key_int = key_int,
            )
        } else {
            format!(
                "bitrate={bitrate} target-usage={tu} rate-control={rc} gop-size={key_int} \
                b-frames=0 ref-frames=1 low-latency=true async-depth=1",
                bitrate = bitrate,
                tu = cfg.qsv_target_usage,
                rc = rc,
                key_int = key_int,
            )
        }
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_d3d11_native(&self) -> bool {
        true
    }
}

pub struct QsvH265Encoder;
impl VideoEncoder for QsvH265Encoder {
    fn gst_element(&self) -> &'static str {
        "qsvh265enc"
    }

    fn codec_name(&self) -> &'static str {
        "h265"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";
        let rc = if is_cqp { "cqp" } else { cfg.rc_mode.as_str() };

        if is_cqp {
            format!(
                "qpi={cqp} qpp={cqp} qpb={cqp} target-usage={tu} rate-control={rc} gop-size={key_int} \
                b-frames=0 ref-frames=1 low-latency=true async-depth=1",
                cqp = cfg.cqp_value,
                tu = cfg.qsv_target_usage,
                rc = rc,
                key_int = key_int,
            )
        } else {
            format!(
                "bitrate={bitrate} target-usage={tu} rate-control={rc} gop-size={key_int} \
                b-frames=0 ref-frames=1 low-latency=true async-depth=1",
                bitrate = bitrate,
                tu = cfg.qsv_target_usage,
                rc = rc,
                key_int = key_int,
            )
        }
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_d3d11_native(&self) -> bool {
        true
    }
}

pub struct AmfEncoder;
impl VideoEncoder for AmfEncoder {
    fn gst_element(&self) -> &'static str {
        "amfh264enc"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";
        let rc = if is_cqp { "cqp" } else { cfg.rc_mode.as_str() };

        format!(
            "bitrate={bitrate} usage=ultralowlatency rc={rc} key-int-max={key_int} b-frames=0",
            bitrate = bitrate,
            rc = rc,
            key_int = key_int,
        )
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_d3d11_native(&self) -> bool {
        true
    }
}

pub struct AmfH265Encoder;
impl VideoEncoder for AmfH265Encoder {
    fn gst_element(&self) -> &'static str {
        "amfh265enc"
    }

    fn codec_name(&self) -> &'static str {
        "h265"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let is_cqp = cfg.rc_mode == "cqp";
        let rc = if is_cqp { "cqp" } else { cfg.rc_mode.as_str() };

        format!(
            "bitrate={bitrate} usage=ultralowlatency rc={rc} key-int-max={key_int} b-frames=0",
            bitrate = bitrate,
            rc = rc,
            key_int = key_int,
        )
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }

    fn is_d3d11_native(&self) -> bool {
        true
    }
}

pub struct MfEncoder;
impl VideoEncoder for MfEncoder {
    fn gst_element(&self) -> &'static str {
        "mfh264enc"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        format!(
            "bitrate={bitrate} rc-mode={rc} low-latency=true",
            bitrate = cfg.bitrate,
            rc = cfg.rc_mode,
        )
    }
}

pub struct MfH265Encoder;
impl VideoEncoder for MfH265Encoder {
    fn gst_element(&self) -> &'static str {
        "mfh265enc"
    }

    fn codec_name(&self) -> &'static str {
        "h265"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        format!(
            "bitrate={bitrate} rc-mode={rc} low-latency=true",
            bitrate = cfg.bitrate,
            rc = cfg.rc_mode,
        )
    }
}