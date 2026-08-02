use crate::pipeline::PlatformContext;

pub fn video_source(_ctx: &PlatformContext, gdi: bool, show_cursor: bool, monitor_index: u32) -> String {
    if gdi {
        // GDI captures the primary virtual screen buffer (all monitors stitched) or primary monitor by default
        "appsrc name=gdi_src format=time is-live=true do-timestamp=true block=false max-bytes=20000000 ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream"
            .to_string()
    } else {
        // Inject monitor-index variable received from the Android Client 3-finger gesture
        format!(
            "d3d11screencapturesrc show-cursor={} monitor-index={} ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream",
            if show_cursor { "true" } else { "false" },
            monitor_index
        )
    }
}

pub fn audio_source() -> String {
    "wasapisrc loopback=true low-latency=true do-timestamp=true".to_string()
}