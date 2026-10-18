// src/pipeline/encoders.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use crate::config::{Capabilities, EncoderChoice, StreamConfig};

pub mod nvenc;
pub mod software;
pub mod vaapi;
pub mod windows_hw;

pub use nvenc::*;
pub use software::*;
pub use vaapi::*;
pub use windows_hw::*;

pub trait VideoEncoder: Send + Sync {
    fn gst_element(&self) -> &'static str;
    fn encode_params(&self, cfg: &StreamConfig) -> String;
    fn codec_name(&self) -> &'static str {
        "h264"
    }
    fn pre_caps(&self) -> Option<&'static str> {
        Some("NV12")
    }
    fn is_hardware(&self) -> bool {
        true
    }
    #[allow(dead_code)]
    fn is_gpu_asic(&self) -> bool {
        false
    }
    #[allow(dead_code)]
    fn is_nvenc(&self) -> bool {
        false
    }
    #[allow(dead_code)]
    fn is_d3d11_native(&self) -> bool {
        false
    }
    #[allow(dead_code)]
    fn is_vaapi_native(&self) -> bool {
        false
    }
    fn is_zero_latency_capable(&self) -> bool {
        true
    }
}

pub fn resolve_encoder(choice: &EncoderChoice, caps: &Capabilities) -> Box<dyn VideoEncoder> {
    let has = |label: &str| caps.encoders.iter().any(|e| e == label);

    match choice {
        EncoderChoice::D3d11H265 if has("d3d11h265") => Box::new(D3d11H265Encoder),
        EncoderChoice::D3d11H264 if has("d3d11h264") => Box::new(D3d11H264Encoder),
        EncoderChoice::NvencH265 if has("nvenc_h265") => Box::new(NvencH265Encoder),
        EncoderChoice::Nvenc if has("nvenc") => Box::new(NvencEncoder),
        EncoderChoice::MfH265 if has("windows_mf_h265") => Box::new(MfH265Encoder),
        EncoderChoice::Mf if has("windows_mf") => Box::new(MfEncoder),
        EncoderChoice::AmfH265 if has("amd_amf_h265") => Box::new(AmfH265Encoder),
        EncoderChoice::Amf if has("amd_amf") => Box::new(AmfEncoder),
        EncoderChoice::QsvH265 if has("intel_qsv_h265") => Box::new(QsvH265Encoder),
        EncoderChoice::Qsv if has("intel_qsv") => Box::new(QsvEncoder),
        EncoderChoice::VaH264 if has("vah264") => Box::new(VaH264Encoder),
        EncoderChoice::VaH265 if has("vah265") => Box::new(VaH265Encoder),
        EncoderChoice::X265 if has("x265") => Box::new(X265Encoder),
        EncoderChoice::X264 if has("x264") => Box::new(X264Encoder),
        EncoderChoice::Auto => {
            #[cfg(target_os = "windows")]
            {
                if has("d3d11h264") {
                    return Box::new(D3d11H264Encoder);
                }
                if has("nvenc") {
                    return Box::new(NvencEncoder);
                }
                if has("amd_amf") {
                    return Box::new(AmfEncoder);
                }
                if has("intel_qsv") {
                    return Box::new(QsvEncoder);
                }
                if has("windows_mf") {
                    return Box::new(MfEncoder);
                }
                if has("d3d11h265") {
                    return Box::new(D3d11H265Encoder);
                }
                if has("nvenc_h265") {
                    return Box::new(NvencH265Encoder);
                }
                if has("amd_amf_h265") {
                    return Box::new(AmfH265Encoder);
                }
                if has("intel_qsv_h265") {
                    return Box::new(QsvH265Encoder);
                }
                if has("windows_mf_h265") {
                    return Box::new(MfH265Encoder);
                }
            }

            #[cfg(target_os = "linux")]
            {
                if has("vah264") {
                    return Box::new(VaH264Encoder);
                }
                if has("vah265") {
                    return Box::new(VaH265Encoder);
                }
                if has("nvenc") {
                    return Box::new(NvencEncoder);
                }
                if has("nvenc_h265") {
                    return Box::new(NvencH265Encoder);
                }
                if has("intel_qsv") {
                    return Box::new(QsvEncoder);
                }
                if has("intel_qsv_h265") {
                    return Box::new(QsvH265Encoder);
                }
            }

            if has("x264") {
                return Box::new(X264Encoder);
            }
            if has("x265") {
                return Box::new(X265Encoder);
            }

            Box::new(X264Encoder)
        }
        _ => {
            tracing::warn!(
                "Requested encoder not available, falling back to verified capability list"
            );
            #[cfg(target_os = "windows")]
            {
                if has("d3d11h264") {
                    return Box::new(D3d11H264Encoder);
                }
                if has("nvenc") {
                    return Box::new(NvencEncoder);
                }
                if has("amd_amf") {
                    return Box::new(AmfEncoder);
                }
                if has("intel_qsv") {
                    return Box::new(QsvEncoder);
                }
                if has("windows_mf") {
                    return Box::new(MfEncoder);
                }
            }

            #[cfg(target_os = "linux")]
            {
                if has("vah264") {
                    return Box::new(VaH264Encoder);
                }
                if has("vah265") {
                    return Box::new(VaH265Encoder);
                }
                if has("nvenc") {
                    return Box::new(NvencEncoder);
                }
                if has("intel_qsv") {
                    return Box::new(QsvEncoder);
                }
            }

            if has("x264") {
                return Box::new(X264Encoder);
            }
            if has("x265") {
                return Box::new(X265Encoder);
            }

            Box::new(X264Encoder)
        }
    }
}