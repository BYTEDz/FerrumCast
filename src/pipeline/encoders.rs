use crate::config::{Capabilities, EncoderChoice, StreamConfig};

pub trait VideoEncoder: Send + Sync {
    /// Returns the name of the GStreamer element (e.g., "qsvh264enc").
    fn gst_element(&self) -> &'static str;

    /// Generates custom rate control and tuning properties for the encoder.
    fn encode_params(&self, cfg: &StreamConfig) -> String;

    /// Returns the target pixel format for input caps (e.g., NV12, I420).
    fn pre_caps(&self) -> Option<&'static str> {
        Some("NV12")
    }

    /// True if this is a hardware-accelerated encoder.
    fn is_hardware(&self) -> bool {
        true
    }

    /// True if this is a dedicated, high-performance GPU hardware ASIC block (QSV, NVENC, AMF).
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
        let bframes = cfg.bframes;
        let is_cqp = cfg.rc_mode == "cqp";

        if is_cqp {
            format!(
                "quantizer={cqp} tune={tune} speed-preset={preset} \
                rc-lookahead=0 sync-lookahead=0 key-int-max={key_int} bframes={bframes} \
                threads=0 sliced-threads=true b-adapt=false \
                option-string=repeat-headers=1",
                cqp = cfg.cqp_value,
                tune = cfg.tune,
                preset = cfg.speed_preset,
                key_int = key_int,
                bframes = bframes,
            )
        } else {
            let vbv = ((bitrate as f32 * 0.05) as u32).max(100);
            format!(
                "bitrate={bitrate} tune={tune} speed-preset={preset} \
                rc-lookahead=0 sync-lookahead=0 key-int-max={key_int} bframes={bframes} \
                threads=0 sliced-threads=true b-adapt=false \
                option-string=nal-hrd=cbr:repeat-headers=1:vbv-maxrate={bitrate}:vbv-bufsize={vbv}",
                bitrate = bitrate,
                tune = cfg.tune,
                preset = cfg.speed_preset,
                key_int = key_int,
                bframes = bframes,
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

pub struct VaH264Encoder;
impl VideoEncoder for VaH264Encoder {
    fn gst_element(&self) -> &'static str {
        "vah264enc"
    }

    fn encode_params(&self, cfg: &StreamConfig) -> String {
        let bitrate = cfg.bitrate;
        let key_int = cfg.key_int_max;
        let bframes = cfg.bframes;
        let ref_frames = cfg.ref_frames;
        let is_cqp = cfg.rc_mode == "cqp";

        if is_cqp {
            format!(
                "rate-control=cqp qp-i={cqp} key-int-max={key_int} \
                target-usage={tu} ref-frames={ref_frames} b-frames={bframes} num-slices=4",
                cqp = cfg.cqp_value,
                key_int = key_int,
                tu = cfg.vaapi_target_usage,
                ref_frames = ref_frames,
                bframes = bframes,
            )
        } else {
            format!(
                "bitrate={bitrate} rate-control={rc} key-int-max={key_int} \
                target-usage={tu} ref-frames={ref_frames} b-frames={bframes} num-slices=4",
                bitrate = bitrate,
                rc = cfg.rc_mode,
                key_int = key_int,
                tu = cfg.vaapi_target_usage,
                ref_frames = ref_frames,
                bframes = bframes,
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
        let bframes = cfg.bframes;
        let ref_frames = cfg.ref_frames;
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
                "qp-const-i={cqp} zerolatency=true preset={preset} tune={tune} \
                rc={rc} key-int-max={key_int} b-frames={bframes} ref={ref_frames}",
                cqp = cfg.cqp_value,
                preset = cfg.nvenc_preset,
                tune = cfg.nvenc_tune,
                rc = rc,
                key_int = key_int,
                bframes = bframes,
                ref_frames = ref_frames,
            )
        } else {
            format!(
                "bitrate={bitrate} zerolatency=true preset={preset} tune={tune} \
                rc={rc} key-int-max={key_int} b-frames={bframes} ref={ref_frames}",
                bitrate = bitrate,
                preset = cfg.nvenc_preset,
                tune = cfg.nvenc_tune,
                rc = rc,
                key_int = key_int,
                bframes = bframes,
                ref_frames = ref_frames,
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
        let bframes = cfg.bframes;
        let ref_frames = cfg.ref_frames;
        let is_cqp = cfg.rc_mode == "cqp";
        let rc = if is_cqp { "cqp" } else { cfg.rc_mode.as_str() };

        if is_cqp {
            format!(
                "qpi={cqp} qpp={cqp} qpb={cqp} target-usage={tu} rate-control={rc} gop-size={key_int} \
                b-frames={bframes} ref-frames={ref_frames} low-latency=true",
                cqp = cfg.cqp_value,
                tu = cfg.qsv_target_usage,
                rc = rc,
                key_int = key_int,
                bframes = bframes,
                ref_frames = ref_frames,
            )
        } else {
            format!(
                "bitrate={bitrate} target-usage={tu} rate-control={rc} gop-size={key_int} \
                b-frames={bframes} ref-frames={ref_frames} low-latency=true",
                bitrate = bitrate,
                tu = cfg.qsv_target_usage,
                rc = rc,
                key_int = key_int,
                bframes = bframes,
                ref_frames = ref_frames,
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
            "bitrate={bitrate} usage=ultralowlatency rc={rc} key-int-max={key_int}",
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

/// Dynamic factory resolving standard system cap mappings to their concrete struct representations.
pub fn resolve_encoder(choice: &EncoderChoice, caps: &Capabilities) -> Box<dyn VideoEncoder> {
    let has = |label: &str| caps.encoders.iter().any(|e| e == label);

    match choice {
        EncoderChoice::Nvenc if has("nvenc") => Box::new(NvencEncoder),
        EncoderChoice::VaH264 if has("vah264") => Box::new(VaH264Encoder),
        EncoderChoice::Qsv if has("intel_qsv") => Box::new(QsvEncoder),
        EncoderChoice::Amf if has("amd_amf") => Box::new(AmfEncoder),
        EncoderChoice::Mf if has("windows_mf") => Box::new(MfEncoder),
        EncoderChoice::X264 => Box::new(X264Encoder),
        EncoderChoice::Auto => {
            if has("nvenc") { return Box::new(NvencEncoder); }
            if has("intel_qsv") { return Box::new(QsvEncoder); }
            if has("amd_amf") { return Box::new(AmfEncoder); }

            #[cfg(target_os = "linux")]
            {
                if has("vah264") { return Box::new(VaH264Encoder); }
            }

            if has("x264") { return Box::new(X264Encoder); }

            #[cfg(target_os = "windows")]
            {
                if has("windows_mf") { return Box::new(MfEncoder); }
            }

            Box::new(X264Encoder)
        }
        _ => {
            tracing::warn!("Requested encoder not available, falling back to best available encoder");
            if has("nvenc") { return Box::new(NvencEncoder); }
            if has("intel_qsv") { return Box::new(QsvEncoder); }
            if has("amd_amf") { return Box::new(AmfEncoder); }

            #[cfg(target_os = "linux")]
            {
                if has("vah264") { return Box::new(VaH264Encoder); }
            }

            if has("x264") { return Box::new(X264Encoder); }

            #[cfg(target_os = "windows")]
            {
                if has("windows_mf") { return Box::new(MfEncoder); }
            }

            Box::new(X264Encoder)
        }
    }
}