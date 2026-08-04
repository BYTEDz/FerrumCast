use crate::config::StreamConfig;

pub fn scale_caps(
    cfg: &StreamConfig,
    format: Option<&str>,
    is_hw: bool,
    mem_feature: Option<&str>,
) -> String {
    let mut pre_elements = String::new();
    let mut parts = Vec::new();

    // Hardware converters (d3d11convert, vapostproc) scale automatically on the GPU.
    // Inserting 'videoscale' breaks zero-copy by forcing a massive CPU memory transfer!
    if (cfg.width.is_some() || cfg.height.is_some()) && !is_hw {
        pre_elements.push_str("videoscale ! ");
    }

    // drop-only=true prevents videorate from duplicating frames and buffering, drastically reducing latency.
    if cfg.framerate.is_some() {
        pre_elements.push_str("videorate drop-only=true ! ");
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

    let format_str = if is_hw {
        format.unwrap_or("NV12")
    } else {
        format.unwrap_or("I420")
    };

    if !is_hw && format_str == "I420" {
        pre_elements.push_str("videoconvert n-threads=0 ! ");
    }

    parts.push(format!("format={}", format_str));
    parts.push(format!("colorimetry={}", cfg.colorimetry));

    let media_type = mem_feature.unwrap_or("video/x-raw");
    let caps_string = format!("{},{} ! ", media_type, parts.join(","));

    format!("{}{}", pre_elements, caps_string)
}