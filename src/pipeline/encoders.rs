use crate::config::{Capabilities, EncoderChoice, StreamConfig};

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
    fn is_gpu_asic(&self) -> bool {
        false
    }
}

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
                "speed-preset={preset} tune={tune} key-int-max={key_int} \
                option-string=qp={cqp}:bframes=0",
                preset = cfg.speed_preset,
                tune = cfg.tune,
                key_int = key_int,
                cqp = cfg.cqp_value,
            )
        } else {
            format!(
                "bitrate={bitrate} speed-preset={preset} tune={tune} key-int-max={key_int} \
                option-string=bframes=0",
                bitrate = bitrate,
                preset = cfg.speed_preset,
                tune = cfg.tune,
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
}

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
                "rate-control=cqp qp-i={cqp} key-int-max={key_int} \
                target-usage={tu} ref-frames=1 b-frames=0 num-slices=4",
                cqp = cfg.cqp_value,
                key_int = key_int,
                tu = cfg.vaapi_target_usage,
            )
        } else {
            format!(
                "bitrate={bitrate} rate-control={rc} key-int-max={key_int} \
                target-usage={tu} ref-frames=1 b-frames=0 num-slices=4",
                bitrate = bitrate,
                rc = cfg.rc_mode,
                key_int = key_int,
                tu = cfg.vaapi_target_usage,
            )
        }
    }

    fn is_gpu_asic(&self) -> bool {
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
                "rate-control=cqp qp-i={cqp} key-int-max={key_int} \
                target-usage={tu} ref-frames=1 b-frames=0 num-slices=4",
                cqp = cfg.cqp_value,
                key_int = key_int,
                tu = cfg.vaapi_target_usage,
            )
        } else {
            format!(
                "bitrate={bitrate} rate-control={rc} key-int-max={key_int} \
                target-usage={tu} ref-frames=1 b-frames=0 num-slices=4",
                bitrate = bitrate,
                rc = cfg.rc_mode,
                key_int = key_int,
                tu = cfg.vaapi_target_usage,
            )
        }
    }

    fn is_gpu_asic(&self) -> bool {
        true
    }
}

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

pub fn resolve_encoder(choice: &EncoderChoice, caps: &Capabilities) -> Box<dyn VideoEncoder> {
    let has = |label: &str| caps.encoders.iter().any(|e| e == label);

    match choice {
        EncoderChoice::NvencH265 if has("nvenc_h265") => Box::new(NvencH265Encoder),
        EncoderChoice::Nvenc if has("nvenc") => Box::new(NvencEncoder),
        EncoderChoice::MfH265 if has("windows_mf_h265") => Box::new(MfH265Encoder),
        EncoderChoice::Mf if has("windows_mf") => Box::new(MfEncoder),
        EncoderChoice::AmfH265 if has("amd_amf_h265") => Box::new(AmfH265Encoder),
        EncoderChoice::Amf if has("amd_amf") => Box::new(AmfEncoder),
        EncoderChoice::QsvH265 if has("intel_qsv_h265") => Box::new(QsvH265Encoder),
        EncoderChoice::Qsv if has("intel_qsv") => Box::new(QsvEncoder),
        EncoderChoice::VaH265 if has("vah265") => Box::new(VaH265Encoder),
        EncoderChoice::VaH264 if has("vah264") => Box::new(VaH264Encoder),
        EncoderChoice::X265 if has("x265") => Box::new(X265Encoder),
        EncoderChoice::X264 => Box::new(X264Encoder),
        EncoderChoice::Auto => {
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
            if has("amd_amf") {
                return Box::new(AmfEncoder);
            }
            if has("amd_amf_h265") {
                return Box::new(AmfH265Encoder);
            }

            #[cfg(target_os = "linux")]
            {
                if has("vah264") {
                    return Box::new(VaH264Encoder);
                }
                if has("vah265") {
                    return Box::new(VaH265Encoder);
                }
            }

            #[cfg(target_os = "windows")]
            {
                if has("windows_mf") {
                    return Box::new(MfEncoder);
                }
                if has("windows_mf_h265") {
                    return Box::new(MfH265Encoder);
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
                "Requested encoder not available, falling back to best available encoder"
            );
            if has("nvenc_h265") {
                return Box::new(NvencH265Encoder);
            }
            if has("nvenc") {
                return Box::new(NvencEncoder);
            }
            if has("intel_qsv_h265") {
                return Box::new(QsvH265Encoder);
            }
            if has("intel_qsv") {
                return Box::new(QsvEncoder);
            }
            if has("amd_amf_h265") {
                return Box::new(AmfH265Encoder);
            }
            if has("amd_amf") {
                return Box::new(AmfEncoder);
            }

            #[cfg(target_os = "linux")]
            {
                if has("vah265") {
                    return Box::new(VaH265Encoder);
                }
                if has("vah264") {
                    return Box::new(VaH264Encoder);
                }
            }

            if has("x265") {
                return Box::new(X265Encoder);
            }
            if has("x264") {
                return Box::new(X264Encoder);
            }

            #[cfg(target_os = "windows")]
            {
                if has("windows_mf_h265") {
                    return Box::new(MfH265Encoder);
                }
                if has("windows_mf") {
                    return Box::new(MfEncoder);
                }
            }

            Box::new(X264Encoder)
        }
    }
}
