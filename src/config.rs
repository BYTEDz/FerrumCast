use gstreamer as gst;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use tracing::{error, info, warn};

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

fn default_speed_preset() -> String {
    "ultrafast".to_string()
}
fn default_tune() -> String {
    "zerolatency".to_string()
}
fn default_nvenc_preset() -> String {
    "p4".to_string()
}
fn default_nvenc_tune() -> String {
    "ultra-low-latency".to_string()
}
fn default_vaapi_target_usage() -> u32 {
    1
}
fn default_qsv_target_usage() -> u32 {
    7
}
fn default_rc_mode() -> String {
    "cbr".to_string()
}
fn default_cqp_value() -> u32 {
    26
}
fn default_key_int_max() -> u32 {
    60
}
fn default_bframes() -> u32 {
    0
}
fn default_ref_frames() -> u32 {
    1
}
fn default_rtp_mtu() -> u32 {
    1200
}
fn default_queue_max_time_ns() -> u64 {
    0
}
fn default_queue_max_buffers() -> u32 {
    1
}
fn default_aggregate_mode() -> String {
    "zero-latency".to_string()
}
fn default_udp_buffer_size() -> u32 {
    2_097_152
}
fn default_show_cursor() -> bool {
    true
}
fn default_colorimetry() -> String {
    "2:3:3:3".to_string()
}
fn default_bitrate() -> u32 {
    6000
}
fn default_client_host() -> String {
    "127.0.0.1".to_string()
}
fn default_audio() -> bool {
    true
}
fn default_audio_only() -> bool {
    false
}
fn default_monitor_index() -> u32 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub framerate: Option<u32>,
    #[serde(default = "default_bitrate")]
    pub bitrate: u32,
    #[serde(default)]
    pub encoder: EncoderChoice,
    #[serde(default = "default_client_host")]
    pub client_host: String,
    #[serde(default = "default_audio")]
    pub audio: bool,
    #[serde(
        default = "default_audio_only",
        rename = "audioOnly",
        alias = "audio_only"
    )]
    pub audio_only: bool,
    pub token: Option<String>,
    #[serde(default)]
    pub gdi: bool,

    #[serde(default = "default_monitor_index")]
    pub monitor_index: u32,

    #[serde(default = "default_speed_preset")]
    pub speed_preset: String,
    #[serde(default = "default_tune")]
    pub tune: String,
    #[serde(default = "default_nvenc_preset")]
    pub nvenc_preset: String,
    #[serde(default = "default_nvenc_tune")]
    pub nvenc_tune: String,
    #[serde(default = "default_vaapi_target_usage")]
    pub vaapi_target_usage: u32,
    #[serde(default = "default_qsv_target_usage")]
    pub qsv_target_usage: u32,

    #[serde(default = "default_rc_mode")]
    pub rc_mode: String,
    #[serde(default = "default_cqp_value")]
    pub cqp_value: u32,

    #[serde(default = "default_key_int_max")]
    pub key_int_max: u32,
    #[serde(default = "default_bframes")]
    pub bframes: u32,
    #[serde(default = "default_ref_frames")]
    pub ref_frames: u32,

    #[serde(default = "default_rtp_mtu")]
    pub rtp_mtu: u32,
    #[serde(default = "default_queue_max_time_ns")]
    pub queue_max_time_ns: u64,
    #[serde(default = "default_queue_max_buffers")]
    pub queue_max_buffers: u32,
    #[serde(default = "default_aggregate_mode")]
    pub aggregate_mode: String,
    #[serde(default = "default_udp_buffer_size")]
    pub udp_buffer_size: u32,

    #[serde(default = "default_show_cursor")]
    pub show_cursor: bool,
    #[serde(default = "default_colorimetry")]
    pub colorimetry: String,

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
            audio_only: false,
            token: None,
            gdi: false,
            monitor_index: default_monitor_index(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub encoders: Vec<String>,
}

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

pub struct ConfigStore(pub RwLock<StreamConfig>);

impl ConfigStore {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self(RwLock::new(StreamConfig::default()))
    }

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
                    } else {
                        warn!("Missing value for --encoder");
                    }
                }
                "--bitrate" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() {
                            cfg.bitrate = v;
                        }
                    }
                }
                "--host" => {
                    if let Some(val) = args.next() {
                        cfg.client_host = val;
                    }
                }
                "--audio" => {
                    if let Some(val) = args.next() {
                        cfg.audio = val != "false";
                    }
                }
                "--audio-only" => {
                    if let Some(val) = args.next() {
                        cfg.audio_only = val != "false";
                    } else {
                        cfg.audio_only = true;
                    }
                }
                "--width" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() {
                            cfg.width = Some(v);
                        }
                    }
                }
                "--height" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() {
                            cfg.height = Some(v);
                        }
                    }
                }
                "--fps" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() {
                            cfg.framerate = Some(v);
                        }
                    }
                }
                "--gdi" => {
                    if let Some(val) = args.next() {
                        cfg.gdi = val != "false";
                    } else {
                        cfg.gdi = true;
                    }
                }
                "--monitor-index" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() {
                            cfg.monitor_index = v;
                        }
                    }
                }
                "--speed-preset" => {
                    if let Some(val) = args.next() {
                        cfg.speed_preset = val;
                    }
                }
                "--tune" => {
                    if let Some(val) = args.next() {
                        cfg.tune = val;
                    }
                }
                "--nvenc-preset" => {
                    if let Some(val) = args.next() {
                        cfg.nvenc_preset = val;
                    }
                }
                "--nvenc-tune" => {
                    if let Some(val) = args.next() {
                        cfg.nvenc_tune = val;
                    }
                }
                "--rc-mode" => {
                    if let Some(val) = args.next() {
                        cfg.rc_mode = val;
                    }
                }
                "--key-int-max" => {
                    if let Some(val) = args.next() {
                        if let Ok(v) = val.parse() {
                            cfg.key_int_max = v;
                        }
                    }
                }
                "--srtp-key" => {
                    if let Some(val) = args.next() {
                        cfg.srtp_key = Some(val);
                    }
                }
                _ => {}
            }
        }
        info!(
            "Pre-seeded config from args: encoder={:?} bitrate={} width={:?} height={:?} fps={:?} \
            gdi={} monitor_index={} rc_mode={} key_int_max={} audio_only={}",
            cfg.encoder,
            cfg.bitrate,
            cfg.width,
            cfg.height,
            cfg.framerate,
            cfg.gdi,
            cfg.monitor_index,
            cfg.rc_mode,
            cfg.key_int_max,
            cfg.audio_only
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
            "Stream config updated: bitrate={}kbps encoder={:?} rc_mode={} monitor_index={} audio_only={}",
            guard.bitrate, guard.encoder, guard.rc_mode, guard.monitor_index, guard.audio_only
        );
    }
}
