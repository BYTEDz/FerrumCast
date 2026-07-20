use crate::pipeline::PlatformContext;

pub fn video_source(_ctx: &PlatformContext, gdi: bool, show_cursor: bool) -> String {
    if gdi {
        // Fallback capture source using GDI BitBlt transfers, where frames are pushed 
        // by a background worker thread into GStreamer via a custom appsrc.
        "appsrc name=gdi_src format=time is-live=true do-timestamp=true block=false max-bytes=20000000 ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream"
            .to_string()
    } else {
        // Hardware-accelerated capture via DXGI Desktop Duplication, retaining 
        // Direct3D 11 textures in GPU memory to maximize throughput.
        format!(
            "d3d11screencapturesrc show-cursor={} ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream ! d3d11convert",
            if show_cursor { "true" } else { "false" }
        )
    }
}

pub fn audio_source() -> String {
    // Target the system loopback interface via WASAPI to record output audio instead of input devices.
    "wasapisrc loopback=true".to_string()
}