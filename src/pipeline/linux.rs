use crate::pipeline::PlatformContext;
use std::env;

pub fn video_source(ctx: &PlatformContext, show_cursor: bool, _monitor_index: u32) -> String {
    if let Some((node_id, fd)) = ctx.portal_info {
        format!(
            "pipewiresrc fd={} path={} do-timestamp=true always-copy=false ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream",
            fd, node_id
        )
    } else if is_wayland() {
        "videotestsrc is-live=true ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream"
            .to_string()
    } else {
        format!(
            "ximagesrc use-damage=true show-pointer={} do-timestamp=true ! \
            queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream",
            if show_cursor { "true" } else { "false" }
        )
    }
}

pub fn audio_source() -> String {
    "pulsesrc buffer-time=10000 latency-time=10000".to_string()
}

pub fn is_wayland() -> bool {
    env::var("WAYLAND_DISPLAY").is_ok()
}