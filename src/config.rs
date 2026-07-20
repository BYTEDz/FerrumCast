use gstreamer as gst;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use tracing::{error, info, warn};

/// Strategies for selecting an H.264 video encoder during pipeline initialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EncoderChoice {
    #[default]
    Auto,
    X264,
    VaH264,
    Nvenc,
    Qsv,
    Amf,
    Mf,
}

fn default_speed_preset() -> String { "ultrafast".to_string() }
fn default_tune() -> String { "zerolatency".to_string() }
fn default_nvenc_preset() -> String { "p4".to_string() }
fn default_nvenc_tune() -> String { "ultra-low-latency".to_string() }
fn default_vaapi_target_usage() -> u32 { 1 }
fn default_qsv_target_usage() -> u32 { 7 }
fn default_rc_mode() -> String { "cbr".to_string() }
fn default_cqp_value() -> u32 { 26 }
fn default_key_int_max() -> u32 { 60 }
fn default_bframes() -> u32 { 0 }
fn default_ref_frames() -> u32 { 1 }
fn default_rtp_mtu() -> u32 { 1200 }
fn default_queue_max_time_ns() -> u64 { 0 }
fn default_queue_max_buffers() -> u32 { 2 }
fn default_aggregate_mode() -> String { "zero-latency".to_string() }
fn default_udp_buffer_size() -> u32 { 2_097_152 }
fn default_show_cursor() -> bool { true }
fn default_colorimetry() -> String { "bt709".to_string() }
fn default_bitrate() -> u32 { 6000 }
fn default_client_host() -> String { "127.0.0.1".to_string() }
fn default_audio() -> bool { true }

/// Runtime configuration parameters representing the active streaming session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Target width in pixels. If None, utilizes the native source resolution (passthrough).
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Target frame rate in FPS. If None, utilizes the native source frame rate (passthrough).
    pub framerate: Option<u32>,
    /// Target encoding bitrate in kilobits per second (kbps).
    #[serde(default = "default_bitrate")]
    pub bitrate: u32,
    #[serde(default)]
    pub encoder: EncoderChoice,
    #[serde(default = "default_client_host")]
    pub client_host: String,
    #[serde(default = "default_audio")]
    pub audio: bool,
    /// Persistent token used to resume screen-capture authorization with the XDG Desktop Portal.
    pub token: Option<String>,
    /// Forces GDI screen capture on Windows, bypassing DXGI desktop duplication (useful for VMs lacking virtual GPUs).
    #[serde(default)]
    pub gdi: bool,

    /// x264 speed-preset: ultrafast, superfast, veryfast, faster, fast, medium, slow, slower, veryslow.
    #[serde(default = "default_speed_preset")]
    pub speed_preset: String,
    /// x264 tune: zerolatency, film, grain, animation, psnr.
    #[serde(default = "default_tune")]
    pub tune: String,
    /// NVENC preset: p1 (fastest) … p7 (best quality).
    #[serde(default = "default_nvenc_preset")]
    pub nvenc_preset: String,
    /// NVENC tune: ultra-low-latency, low-latency, high-quality.
    #[serde(default = "default_nvenc_tune")]
    pub nvenc_tune: String,
    /// VA-API target-usage: 1 (fastest) … 7 (best quality).
    #[serde(default = "default_vaapi_target_usage")]
    pub vaapi_target_usage: u32,
    /// QSV target-usage: 1 (fastest) … 7 (balanced).
    #[serde(default = "default_qsv_target_usage")]
    pub qsv_target_usage: u32,

    /// Rate control mode: cbr, vbr, cqp.
    #[serde(default = "default_rc_mode")]
    pub rc_mode: String,
    /// Constant quantization parameter (0–51). Active only when rc_mode = "cqp".
    #[serde(default = "default_cqp_value")]
    pub cqp_value: u32,

    /// Keyframe interval in frames (1–300).
    #[serde(default = "default_key_int_max")]
    pub key_int_max: u32,

    /// Number of B-frames (0 = disabled, low latency; 1-2 for better compression).
    #[serde(default = "default_bframes")]
    pub bframes: u32,
    /// Number of reference frames (1 = low latency; up to 4 for quality).
    #[serde(default = "default_ref_frames")]
    pub ref_frames: u32,

    /// RTP packet MTU in bytes (1000–1500).
    #[serde(default = "default_rtp_mtu")]
    pub rtp_mtu: u32,
    /// Queue max-size-time in nanoseconds. 0 = disabled. E.g. 100_000_000 = 100 ms latency cap.
    #[serde(default = "default_queue_max_time_ns")]
    pub queue_max_time_ns: u64,
    /// Queue max-size-buffers (1–30).
    #[serde(default = "default_queue_max_buffers")]
    pub queue_max_buffers: u32,
    /// rtph264pay aggregate-mode: zero-latency, none, next-keyframe.
    #[serde(default = "default_aggregate_mode")]
    pub aggregate_mode: String,
    /// UDP send buffer size in bytes.
    #[serde(default = "default_udp_buffer_size")]
    pub udp_buffer_size: u32,

    /// Show cursor in X11 ximagesrc captures.
    #[serde(default = "default_show_cursor")]
    pub show_cursor: bool,
    /// GStreamer colorimetry tag: bt709, bt601, bt2020.
    #[serde(default = "default_colorimetry")]
    pub colorimetry: String,

    /// Optional SRTP Master key and salt (concatenated hex string)
    #[serde(default)]
    pub srtp_key: Option<String>,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            framerate: None,
            bitrate: default_bitrate(),
            encoder: EncoderChoice::Auto,
            client_host: default_client_host(),
            audio: true,
            token: None,
            gdi: false,
            speed_preset: default_speed_preset(),
            tune: default_tune(),
            nvenc_preset: default_nvenc_preset(),
            nvenc_tune: default_nvenc_tune(),
            vaapi_target_usage: default_vaapi_target_usage(),
            qsv_target_usage: default_qsv_target_usage(),
            rc_mode: default_rc_mode(),
            cqp_value: default_cqp_value(),
            key_int_max: default_key_int_max(),
            bframes: default_bframes(),
            ref_frames: default_ref_frames(),
            rtp_mtu: default_rtp_mtu(),
            queue_max_time_ns: default_queue_max_time_ns(),
            queue_max_buffers: default_queue_max_buffers(),
            aggregate_mode: default_aggregate_mode(),
            udp_buffer_size: default_udp_buffer_size(),
            show_cursor: default_show_cursor(),
            colorimetry: default_colorimetry(),
            srtp_key: None,
        }
    }
}

/// Hardware and software H.264 compression capabilities detected on the current host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub encoders: Vec<String>,
}

/// Probes the local GStreamer plugin registry to identify installed H.264 encoder elements.
pub fn probe_capabilities() -> Capabilities {
    let mut encoders = Vec::new();

    let candidates = [
        ("nvh264enc", "nvenc"),
        ("mfh264enc", "windows_mf"),
        ("amfh264enc", "amd_amf"),
        ("qsvh264enc", "intel_qsv"),
        ("vah264enc", "vah264"),
        ("x264enc", "x264"),
    ];

    for (element, label) in &candidates {
        if gst::ElementFactory::make(element).build().is_ok() {
            info!("encoder available and instantiatable: {}", label);
            encoders.push(label.to_string());
        } else {
            warn!(
                "encoder factory found but failed to instantiate: {}",
                element
            );
        }
    }

    if encoders.is_empty() {
        warn!("No hardware encoders found, forcing x264 fallback");
        encoders.push("x264".to_string());
    }

    Capabilities { encoders }
}

/// A thread-safe configuration repository, permitting concurrent readers
/// and synchronized updates across async tasks.
pub struct ConfigStore(pub RwLock<StreamConfig>);

impl ConfigStore {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self(RwLock::new(StreamConfig::default()))
    }

    /// Parses CLI arguments to initialize config defaults.
    pub fn new_from_args() -> Self {
        let mut cfg = StreamConfig::default();
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--encoder" => {
                    if let Some(val) = args.next() {
                        cfg.encoder = match val.to_lowercase().as_str() {
                            "vah264" => EncoderChoice::VaH264,
                            "nvenc" => EncoderChoice::Nvenc,
                            "qsv" => EncoderChoice::Qsv,
                            "amf" => EncoderChoice::Amf,
                            "mf" => EncoderChoice::Mf,
                            "x264" => EncoderChoice::X264,
                            _ => EncoderChoice::Auto,
                        };
                    } else { warn!("Missing value for --encoder"); }
                }
                "--bitrate" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.bitrate = v; }
                        else { warn!("Invalid bitrate value"); }
                    } else { warn!("Missing value for --bitrate"); }
                }
                "--host" => {
                    if let Some(val) = args.next() { cfg.client_host = val; }
                    else { warn!("Missing value for --host"); }
                }
                "--audio" => {
                    if let Some(val) = args.next() { cfg.audio = val != "false"; }
                    else { warn!("Missing value for --audio"); }
                }
                "--width" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.width = Some(v); }
                        else { warn!("Invalid width value"); }
                    } else { warn!("Missing value for --width"); }
                }
                "--height" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.height = Some(v); }
                        else { warn!("Invalid height value"); }
                    } else { warn!("Missing value for --height"); }
                }
                "--fps" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.framerate = Some(v); }
                        else { warn!("Invalid fps value"); }
                    } else { warn!("Missing value for --fps"); }
                }
                "--gdi" => {
                    if let Some(val) = args.next() { cfg.gdi = val != "false"; }
                    else { cfg.gdi = true; }
                }
                "--speed-preset" => {
                    if let Some(val) = args.next() { cfg.speed_preset = val; }
                    else { warn!("Missing value for --speed-preset"); }
                }
                "--tune" => {
                    if let Some(val) = args.next() { cfg.tune = val; }
                    else { warn!("Missing value for --tune"); }
                }
                "--nvenc-preset" => {
                    if let Some(val) = args.next() { cfg.nvenc_preset = val; }
                    else { warn!("Missing value for --nvenc-preset"); }
                }
                "--nvenc-tune" => {
                    if let Some(val) = args.next() { cfg.nvenc_tune = val; }
                    else { warn!("Missing value for --nvenc-tune"); }
                }
                "--vaapi-target-usage" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.vaapi_target_usage = v; }
                        else { warn!("Invalid vaapi-target-usage value"); }
                    } else { warn!("Missing value for --vaapi-target-usage"); }
                }
                "--qsv-target-usage" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.qsv_target_usage = v; }
                        else { warn!("Invalid qsv-target-usage value"); }
                    } else { warn!("Missing value for --qsv-target-usage"); }
                }
                "--rc-mode" => {
                    if let Some(val) = args.next() { cfg.rc_mode = val; }
                    else { warn!("Missing value for --rc-mode"); }
                }
                "--cqp-value" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.cqp_value = v; }
                        else { warn!("Invalid cqp-value"); }
                    } else { warn!("Missing value for --cqp-value"); }
                }
                "--key-int-max" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.key_int_max = v; }
                        else { warn!("Invalid key-int-max value"); }
                    } else { warn!("Missing value for --key-int-max"); }
                }
                "--bframes" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.bframes = v; }
                        else { warn!("Invalid bframes value"); }
                    } else { warn!("Missing value for --bframes"); }
                }
                "--ref-frames" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.ref_frames = v; }
                        else { warn!("Invalid ref-frames value"); }
                    } else { warn!("Missing value for --ref-frames"); }
                }
                "--rtp-mtu" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.rtp_mtu = v; }
                        else { warn!("Invalid rtp-mtu value"); }
                    } else { warn!("Missing value for --rtp-mtu"); }
                }
                "--queue-max-time-ns" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.queue_max_time_ns = v; }
                        else { warn!("Invalid queue-max-time-ns value"); }
                    } else { warn!("Missing value for --queue-max-time-ns"); }
                }
                "--queue-max-buffers" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.queue_max_buffers = v; }
                        else { warn!("Invalid queue-max-buffers value"); }
                    } else { warn!("Missing value for --queue-max-buffers"); }
                }
                "--aggregate-mode" => {
                    if let Some(val) = args.next() { cfg.aggregate_mode = val; }
                    else { warn!("Missing value for --aggregate-mode"); }
                }
                "--udp-buffer-size" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() { cfg.udp_buffer_size = v; }
                        else { warn!("Invalid udp-buffer-size value"); }
                    } else { warn!("Missing value for --udp-buffer-size"); }
                }
                "--show-cursor" => {
                    if let Some(val) = args.next() { cfg.show_cursor = val != "false"; }
                    else { cfg.show_cursor = true; }
                }
                "--colorimetry" => {
                    if let Some(val) = args.next() { cfg.colorimetry = val; }
                    else { warn!("Missing value for --colorimetry"); }
                }
                "--srtp-key" => {
                    if let Some(val) = args.next() { cfg.srtp_key = Some(val); }
                    else { warn!("Missing value for --srtp-key"); }
                }
                _ => {}
            }
        }
        info!(
            "Pre-seeded config from args: encoder={:?} bitrate={} width={:?} height={:?} fps={:?} \
            gdi={} token={:?} rc_mode={} key_int_max={} bframes={}",
            cfg.encoder, cfg.bitrate, cfg.width, cfg.height, cfg.framerate,
            cfg.gdi, cfg.token, cfg.rc_mode, cfg.key_int_max, cfg.bframes
        );
        Self(RwLock::new(cfg))
    }

    pub fn get(&self) -> StreamConfig {
        self.0
            .read()
            .unwrap_or_else(|e| {
                error!("ConfigStore RwLock was poisoned! Recovering with dirty values.");
                e.into_inner()
            })
            .clone()
    }

    pub fn set(&self, cfg: StreamConfig) {
        let mut guard = self.0.write().unwrap_or_else(|e| {
            error!("ConfigStore RwLock was poisoned on write! Attempting recovery.");
            e.into_inner()
        });
        *guard = cfg;
        info!(
            "Stream config updated: bitrate={}kbps encoder={:?} rc_mode={} key_int_max={}",
            guard.bitrate, guard.encoder, guard.rc_mode, guard.key_int_max
        );
    }
}