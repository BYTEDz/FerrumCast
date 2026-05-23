use crate::pipeline::PlatformContext;
use std::env;

pub fn video_source(ctx: &PlatformContext, show_cursor: bool) -> String {
    if let Some((node_id, fd)) = ctx.portal_info {
        // Bind the PipeWire source to the file descriptor and node ID negotiated via the XDG Desktop Portal.
        format!(
            "pipewiresrc fd={} path={} do-timestamp=true ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream",
            fd, node_id
        )
    } else if is_wayland() {
        // Direct display capture is prohibited under Wayland without portal authorization.
        // Fall back to a test pattern source to prevent pipeline construction failure.
        "videotestsrc is-live=true ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream"
            .to_string()
    } else {
        // Capture frames directly from the X11 root window via ximagesrc, utilizing XDamage to optimize redraw updates.
        format!(
            "ximagesrc use-damage=true show-pointer={show_cursor} do-timestamp=true ! \
            queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream",
            show_cursor = if show_cursor { "true" } else { "false" },
        )
    }
}

pub fn audio_source() -> String {
    "pulsesrc".to_string()
}

pub fn is_wayland() -> bool {
    env::var("WAYLAND_DISPLAY").is_ok()
}