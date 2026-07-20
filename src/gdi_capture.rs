#[cfg(target_os = "windows")]
use gstreamer as gst;
#[cfg(target_os = "windows")]
use gstreamer_app as gst_app;
#[cfg(target_os = "windows")]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[cfg(target_os = "windows")]
use std::thread;
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
pub fn start_gdi_capture(appsrc: gst_app::AppSrc) -> Arc<AtomicBool> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    thread::spawn(move || {
        unsafe {
            let h_dc_src = GetDC(HWND(0));
            if h_dc_src.0 == 0 {
                tracing::error!("GetDC failed");
                return;
            }

            let scr_width = GetSystemMetrics(SM_CXSCREEN);
            let scr_height = GetSystemMetrics(SM_CYSCREEN);

            tracing::info!("Custom GDI Capture active: {}x{}", scr_width, scr_height);

            let h_dc_mem = CreateCompatibleDC(h_dc_src);
            let h_bmp = CreateCompatibleBitmap(h_dc_src, scr_width, scr_height);
            SelectObject(h_dc_mem, h_bmp);

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = scr_width;
            bmi.bmiHeader.biHeight = -scr_height;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = 0; // BI_RGB (0) specifies uncompressed bitmap layout.

            let row_size = (scr_width * 4) as usize;
            let buffer_size = row_size * (scr_height as usize);
            let mut buffer = vec![0u8; buffer_size];

            let mut pts = 0;
            // Target frame duration for ~30 FPS in nanoseconds (33.33ms).
            let duration = 33_333_333;

            appsrc.set_caps(Some(
                &gst::Caps::builder("video/x-raw")
                    .field("format", "BGRA")
                    .field("width", scr_width)
                    .field("height", scr_height)
                    .field("framerate", gst::Fraction::new(30, 1))
                    .build(),
            ));

            while r.load(Ordering::SeqCst) {
                if BitBlt(
                    h_dc_mem, 0, 0, scr_width, scr_height, h_dc_src, 0, 0, SRCCOPY,
                )
                .is_err()
                {
                    break;
                }

                // Overlay the system cursor onto the back-buffer context.
                let mut cursor_info: CURSORINFO = std::mem::zeroed();
                cursor_info.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
                if GetCursorInfo(&mut cursor_info).is_ok() && cursor_info.flags == CURSOR_SHOWING {
                    let cx = cursor_info.ptScreenPos.x;
                    let cy = cursor_info.ptScreenPos.y;

                    // Adjust for cursor hotspot offsets to align the pointer tip with absolute coordinates.
                    let mut icon_info: ICONINFO = std::mem::zeroed();
                    if GetIconInfo(cursor_info.hCursor, &mut icon_info).is_ok() {
                        let hx = icon_info.xHotspot as i32;
                        let hy = icon_info.yHotspot as i32;

                        // Release bitmaps allocated by GetIconInfo to prevent GDI handle leaks.
                        if !icon_info.hbmColor.is_invalid() {
                            let _ = DeleteObject(icon_info.hbmColor);
                        }
                        if !icon_info.hbmMask.is_invalid() {
                            let _ = DeleteObject(icon_info.hbmMask);
                        }

                        let _ = DrawIconEx(
                            h_dc_mem,
                            cx - hx,
                            cy - hy,
                            cursor_info.hCursor,
                            0,
                            0,
                            0,
                            None,
                            DI_NORMAL,
                        );
                    }
                }
                if GetDIBits(
                    h_dc_mem,
                    h_bmp,
                    0,
                    scr_height as u32,
                    Some(buffer.as_mut_ptr() as *mut _),
                    &mut bmi,
                    DIB_RGB_COLORS,
                ) == 0
                {
                    break;
                }

                let mut out_buf = gst::Buffer::with_size(buffer_size).unwrap();
                {
                    let mut map = out_buf.get_mut().unwrap().map_writable().unwrap();
                    map.as_mut_slice().copy_from_slice(&buffer);
                }

                out_buf
                    .get_mut()
                    .unwrap()
                    .set_pts(gst::ClockTime::from_nseconds(pts));
                out_buf
                    .get_mut()
                    .unwrap()
                    .set_duration(gst::ClockTime::from_nseconds(duration));
                pts += duration;

                if appsrc.push_buffer(out_buf).is_err() {
                    break;
                }

                // Throttle loop execution to target a ~30 FPS capture rate.
                thread::sleep(Duration::from_millis(33));
            }

            DeleteObject(h_bmp);
            DeleteDC(h_dc_mem);
            ReleaseDC(HWND(0), h_dc_src);
        }
    });

    running
}