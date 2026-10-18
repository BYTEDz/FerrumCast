// src/pipeline/transport.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use crate::config::StreamConfig;
use crate::pipeline::encoders::VideoEncoder;

pub const DEFAULT_VIDEO_RTP_PORT: u16 = 5004;
pub const DEFAULT_AUDIO_RTP_PORT: u16 = 5006;
pub const AUDIO_UDP_BUFFER_SIZE: u32 = 1_048_576;

pub fn format_multiudpsink(client_hosts_raw: &str, port: u16, buffer_size: u32) -> String {
    let hosts: Vec<&str> = client_hosts_raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if hosts.is_empty() {
        format!(
            "multiudpsink clients=\"127.0.0.1:{}\" sync=false async=false buffer-size={}",
            port, buffer_size
        )
    } else {
        let clients: Vec<String> = hosts
            .iter()
            .map(|h| {
                if h.contains(':') {
                    h.to_string()
                } else {
                    format!("{}:{}", h, port)
                }
            })
            .collect();
        format!(
            "multiudpsink clients=\"{}\" sync=false async=false buffer-size={}",
            clients.join(","),
            buffer_size
        )
    }
}

pub fn apply_srtp_encryption(fragment: String, srtp_key: Option<&str>) -> String {
    if let Some(key) = srtp_key {
        format!(
            "{} ! srtpenc key=\"{}\" rtp-cipher=aes-128-icm rtp-auth=hmac-sha1-80 rtcp-cipher=aes-128-icm rtcp-auth=hmac-sha1-80",
            fragment, key
        )
    } else {
        fragment
    }
}

pub fn build_video_transport(
    encoded_source: &str,
    enc: &dyn VideoEncoder,
    cfg: &StreamConfig,
) -> String {
    let (qbufs, qtime, leaky) = if enc.is_zero_latency_capable() {
        (
            cfg.queue_max_buffers.max(1),
            cfg.queue_max_time_ns,
            "leaky=downstream",
        )
    } else {
        (3u32, 33_000_000u64, "")
    };

    let codec = enc.codec_name();
    let rtp_pay = if codec == "h265" {
        format!(
            "video/x-h265 ! h265parse config-interval=-1 disable-passthrough=true ! \
            video/x-h265,stream-format=byte-stream,alignment=au ! \
            queue max-size-buffers={qbufs} max-size-bytes=0 max-size-time={qtime} {leaky} ! \
            rtph265pay mtu={mtu} config-interval=1 pt=96 aggregate-mode=none",
            qbufs = qbufs,
            qtime = qtime,
            leaky = leaky,
            mtu = cfg.rtp_mtu,
        )
    } else {
        format!(
            "video/x-h264 ! h264parse config-interval=-1 disable-passthrough=true ! \
            video/x-h264,stream-format=byte-stream,alignment=au ! \
            queue max-size-buffers={qbufs} max-size-bytes=0 max-size-time={qtime} {leaky} ! \
            rtph264pay mtu={mtu} config-interval=1 pt=96 aggregate-mode=none",
            qbufs = qbufs,
            qtime = qtime,
            leaky = leaky,
            mtu = cfg.rtp_mtu,
        )
    };

    let video_stream = format!("{} ! {}", encoded_source, rtp_pay);
    let encrypted = apply_srtp_encryption(video_stream, cfg.srtp_key.as_deref());
    let sink = format_multiudpsink(&cfg.client_host, DEFAULT_VIDEO_RTP_PORT, cfg.udp_buffer_size);

    format!("{} ! {}", encrypted, sink)
}

pub fn build_audio_branch(audio_source: &str, cfg: &StreamConfig) -> String {
    let audio_bitrate = cfg.audio_bitrate * 1000;
    let audio_pipeline = format!(
        "{} ! queue max-size-buffers=5 max-size-bytes=0 max-size-time=0 leaky=downstream ! \
        audioconvert ! audioresample ! opusenc inband-fec=true frame-size=10 audio-type=2051 bitrate={} ! rtpopuspay",
        audio_source, audio_bitrate
    );

    let encrypted = apply_srtp_encryption(audio_pipeline, cfg.srtp_key.as_deref());
    let sink = format_multiudpsink(&cfg.client_host, DEFAULT_AUDIO_RTP_PORT, AUDIO_UDP_BUFFER_SIZE);

    format!(" {} ! {}", encrypted, sink)
}

pub fn build_standalone_audio_pipeline(audio_source: &str, cfg: &StreamConfig) -> String {
    let audio_bitrate = cfg.audio_bitrate * 1000;
    let audio_pipeline = format!(
        "{} ! queue max-size-buffers=5 max-size-bytes=0 max-size-time=0 leaky=downstream ! \
        audioconvert ! audioresample ! opusenc inband-fec=true frame-size=10 audio-type=2051 bitrate={} ! rtpopuspay",
        audio_source, audio_bitrate
    );

    let encrypted = apply_srtp_encryption(audio_pipeline, cfg.srtp_key.as_deref());
    let sink = format_multiudpsink(&cfg.client_host, DEFAULT_AUDIO_RTP_PORT, cfg.udp_buffer_size);

    format!("{} ! {}", encrypted, sink)
}