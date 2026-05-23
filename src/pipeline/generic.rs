use crate::config::StreamConfig;

pub fn scale_caps(
    cfg: &StreamConfig,
    format: Option<&str>,
    is_hw: bool,
    mem_feature: Option<&str>,
    skip_videoscale: bool,
) -> String {
    let mut pre_elements = String::new();
    let mut parts = Vec::new();

    // Conditionally insert scaling and frame-rate conversion elements only when spatial
    // or temporal modifications are explicitly requested to minimize pipeline overhead.
    if cfg.width.is_some() || cfg.height.is_some() {
        if !skip_videoscale {
            pre_elements.push_str("videoscale ! ");
        }
    }
    if cfg.framerate.is_some() {
        pre_elements.push_str("videorate ! ");
    }

    if let Some(fps) = cfg.framerate {
        parts.push(format!("framerate={}/1", fps));
    }
    if let Some(w) = cfg.width {
        parts.push(format!("width={}", w));
    }
    if let Some(h) = cfg.height {
        parts.push(format!("height={}", h));
    }

    // Select the target pixel format based on the acceleration context. Hardware
    // encoders typically expect NV12, whereas software fallback paths use I420.
    let format_str = if is_hw {
        format.unwrap_or("NV12")
    } else {
        format.unwrap_or("I420")
    };

    // Inject multi-threaded conversion (n-threads=0) when targeting software-based planar formats.
    if format_str == "I420" {
        pre_elements.push_str("videoconvert n-threads=0 ! ");
    }

    // Enforce explicit format capability constraints to guarantee successful downstream caps negotiation.
    parts.push(format!("format={}", format_str));

    // Use the configured colorimetry (default bt709) to guarantee correct color-space
    // mapping by hardware converters and ensure encoders embed appropriate VUI metadata.
    parts.push(format!("colorimetry={}", cfg.colorimetry));

    let media_type = mem_feature.unwrap_or("video/x-raw");
    let caps_string = format!("{},{} ! ", media_type, parts.join(","));

    format!("{}{}", pre_elements, caps_string)
}