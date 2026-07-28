use crate::pipeline::PlatformContext;

pub fn video_source(_ctx: &PlatformContext, gdi: bool, show_cursor: bool) -> String {
    if gdi {
        "appsrc name=gdi_src format=time is-live=true do-timestamp=true block=false max-bytes=20000000 ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream"
            .to_string()
    } else {
        format!(
            "d3d11screencapturesrc show-cursor={} ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream ! d3d11convert",
            if show_cursor { "true" } else { "false" }
        )
    }
}

pub fn audio_source() -> String {
    // Enable low-latency and do-timestamp on WASAPI loopback to prevent driver callback starvation
    "wasapisrc loopback=true low-latency=true do-timestamp=true".to_string()
}