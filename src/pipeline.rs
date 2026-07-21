use crate::config::{Capabilities, StreamConfig};
use tracing::info;

pub mod encoders;

#[derive(Debug, Clone, Default)]
pub struct PlatformContext {
    #[cfg(target_os = "linux")]
    pub portal_info: Option<(u32, i32)>,
}

pub struct PipelineBuilder;

impl PipelineBuilder {
    pub fn build_pipeline(
        cfg: &StreamConfig,
        enc: &dyn encoders::VideoEncoder,
        ctx: &PlatformContext,
    ) -> String {
        let is_hw = enc.is_hardware();
        let is_vm = detect_hypervisor();

        // On Windows-based virtual machines lacking GPU passthrough, DXGI-based desktop duplication
        // (d3d11screencapturesrc) yields a black frame. Fall back to GDI screen capture under these
        // hypervisor conditions when software-based fallback encoders are selected.
        let is_hw_encoder = enc.is_gpu_asic();
        let use_gdi = cfg.gdi || (is_vm && !is_hw_encoder);

        if use_gdi && cfg!(target_os = "windows") {
            info!("Utilizing GDI screen capturing (Virtual machine / VM mode active)");
        }

        #[cfg(target_os = "windows")]
        let video_src = self::sys::video_source(ctx, use_gdi, cfg.show_cursor);
        #[cfg(not(target_os = "windows"))]
        let video_src = self::sys::video_source(ctx, cfg.show_cursor);

        let (converter, caps) = if cfg!(target_os = "windows") {
            if use_gdi {
                // GDI capture buffers reside in host system memory (BGR), bypassing the need
                // for a Direct3D 11 download stage.
                let conv = "videoconvert n-threads=0".to_string();
                let c = self::generic::scale_caps(cfg, enc.pre_caps(), is_hw, None, false);
                (conv, c)
            } else {
                if enc.is_gpu_asic() {
                    // Bind the pipeline to the GPU memory domain to maintain hardware-accelerated execution.
                    let conv = "d3d11convert".to_string();
                    let mem_feature = Some("video/x-raw(memory:D3D11Memory)");
                    let c =
                        self::generic::scale_caps(cfg, enc.pre_caps(), is_hw, mem_feature, true);
                    (conv, c)
                } else {
                    // GPU-Accelerated Scaling & Color Conversion:
                    // Perform resizing and NV12 color conversion entirely on the GPU inside D3D11 Memory.
                    let target_format = enc.pre_caps().unwrap_or("NV12");
                    let gpu_mem_feature = Some("video/x-raw(memory:D3D11Memory)");
                    let gpu_caps = self::generic::scale_caps(
                        cfg,
                        Some(target_format),
                        true,
                        gpu_mem_feature,
                        true,
                    );

                    let conv = format!(
                        "d3d11convert ! {}d3d11download ! videoconvert n-threads=0",
                        gpu_caps
                    );
                    let c = format!("video/x-raw,format={} ! ", target_format);
                    (conv, c)
                }
            }
        } else {
            // Linux path
            let mem_feature = if enc.gst_element() == "vah264enc" {
                Some("video/x-raw(memory:VAMemory)")
            } else if enc.gst_element() == "nvh264enc" {
                Some("video/x-raw(memory:GLMemory)")
            } else {
                None
            };

            let conv = if enc.gst_element() == "vah264enc" {
                "vapostproc".to_string()
            } else if enc.gst_element() == "nvh264enc" {
                "glcolorconvert".to_string()
            } else {
                "videoconvert n-threads=0".to_string()
            };

            let c = self::generic::scale_caps(cfg, enc.pre_caps(), is_hw, mem_feature, false);
            (conv, c)
        };

        let qbufs = cfg.queue_max_buffers;
        let qtime = cfg.queue_max_time_ns;

        // Configure 'config-interval' to -1 on both the parser and payloader. This forces GStreamer
        // to inline SPS/PPS parameter sets with every IDR keyframe, allowing newly joined clients
        // to decode the stream immediately without waiting for a renegotiation cycle.
        //
        // Configure leaky queues with configurable buffer limits. For real-time interactive streaming,
        // dropping older frames (leaky=downstream) is preferred over introducing queuing delays
        // during transient network congestion.
        let mut video = format!(
            "{video_src} ! {converter} ! {caps}{enc_element} name=video_encoder {enc_params} ! \
            video/x-h264,profile=constrained-baseline ! h264parse config-interval=-1 ! \
            video/x-h264,stream-format=byte-stream,alignment=au ! \
            queue max-size-buffers={qbufs} max-size-bytes=0 max-size-time={qtime} leaky=downstream ! \
            rtph264pay mtu={mtu} config-interval=-1 pt=96 aggregate-mode={agg}",
            video_src = video_src,
            converter = converter,
            caps = caps,
            enc_element = enc.gst_element(),
            enc_params = enc.encode_params(cfg),
            mtu = cfg.rtp_mtu,
            agg = cfg.aggregate_mode,
        );

        let mut audio = if cfg.audio {
            let src = self::sys::audio_source();
            format!(
                "{} ! queue max-size-buffers=5 max-size-bytes=0 max-size-time=0 leaky=downstream ! \
                audioconvert ! audioresample ! opusenc ! rtpopuspay",
                src
            )
        } else {
            String::new()
        };

        // Apply SRTP if key is provided
        if let Some(ref srtp_key) = cfg.srtp_key {
            video = format!(
                "{} ! srtpenc key=\"{}\" rtp-cipher=aes-128-icm rtp-auth=hmac-sha1-80 rtcp-cipher=aes-128-icm rtcp-auth=hmac-sha1-80",
                video, srtp_key
            );
            if cfg.audio {
                audio = format!(
                    "{} ! srtpenc key=\"{}\" rtp-cipher=aes-128-icm rtp-auth=hmac-sha1-80 rtcp-cipher=aes-128-icm rtcp-auth=hmac-sha1-80",
                    audio, srtp_key
                );
            }
        }

        let udp_buf = cfg.udp_buffer_size;

        let audio_branch = if cfg.audio {
            format!(
                " {} ! udpsink host={} port=5006 sync=false async=false buffer-size=1048576",
                audio, cfg.client_host
            )
        } else {
            String::new()
        };

        format!(
            "{} ! udpsink host={} port=5004 sync=false async=false buffer-size={}{}",
            video, cfg.client_host, udp_buf, audio_branch
        )
    }

    #[cfg(target_os = "linux")]
    pub fn is_wayland() -> bool {
        self::sys::is_wayland()
    }

    pub fn probe_capabilities() -> Capabilities {
        crate::config::probe_capabilities()
    }
}

/// Queries the x86 CPUID register space to detect if the process is executing within a hypervisor environment.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn detect_hypervisor() -> bool {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::__cpuid;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::__cpuid;

    let res = __cpuid(1);
    // Bit 31 of the ECX register indicates hypervisor presence on CPUID leaf 1.
    (res.ecx & (1 << 31)) != 0
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn detect_hypervisor() -> bool {
    false
}

mod generic;

#[cfg(target_os = "linux")]
#[path = "pipeline/linux.rs"]
mod sys;

#[cfg(target_os = "windows")]
#[path = "pipeline/windows.rs"]
mod sys;
