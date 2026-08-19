// src/pipeline.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use crate::config::{Capabilities, StreamConfig};
use crate::platform::PlatformBackend;
use tracing::info;

pub mod encoders;
pub mod generic;

pub struct PipelineBuilder;

fn format_multiudpsink(client_hosts_raw: &str, port: u16, buffer_size: u32) -> String {
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

impl PipelineBuilder {
    pub fn build_pipeline(
        cfg: &StreamConfig,
        enc: &dyn encoders::VideoEncoder,
        platform: &dyn PlatformBackend,
    ) -> String {
        let audio_bitrate = cfg.audio_bitrate * 1000;
        let udp_buf = cfg.udp_buffer_size;

        if cfg.audio_only {
            info!(
                "Building Audio-Only multi-client unicast stream pipeline (clients={}, audio_bitrate={}bps)",
                cfg.client_host, audio_bitrate
            );
            let src = platform.build_audio_source(cfg);
            let mut audio = format!(
                "{} ! queue max-size-buffers=5 max-size-bytes=0 max-size-time=0 leaky=downstream ! \
                audioconvert ! audioresample ! opusenc inband-fec=true frame-size=10 audio-type=2051 bitrate={} ! rtpopuspay",
                src, audio_bitrate
            );

            if let Some(ref srtp_key) = cfg.srtp_key {
                audio = format!(
                    "{} ! srtpenc key=\"{}\" rtp-cipher=aes-128-icm rtp-auth=hmac-sha1-80 rtcp-cipher=aes-128-icm rtcp-auth=hmac-sha1-80",
                    audio, srtp_key
                );
            }

            let audio_sink = format_multiudpsink(&cfg.client_host, 5006, udp_buf);
            return format!("{} ! {}", audio, audio_sink);
        }

        let is_hw = enc.is_hardware();
        let video_desc = platform.build_video_source(cfg, enc.is_gpu_asic());

        let base_caps = generic::scale_caps(
            cfg,
            enc.pre_caps(),
            is_hw,
            video_desc.preferred_memory_feature,
        );

        let caps = if let Some(ref raw_filter) = video_desc.raw_caps_filter {
            format!("{}{}", raw_filter, base_caps)
        } else {
            base_caps
        };

        let qbufs = cfg.queue_max_buffers;
        let qtime = cfg.queue_max_time_ns;
        let codec = enc.codec_name();

        let mut video = if codec == "h265" {
            format!(
                "{video_src} ! {converter} ! {caps}{enc_element} name=video_encoder {enc_params} ! \
                video/x-h265 ! h265parse config-interval=-1 disable-passthrough=true ! \
                video/x-h265,stream-format=byte-stream,alignment=au ! \
                queue max-size-buffers={qbufs} max-size-bytes=0 max-size-time={qtime} ! \
                rtph265pay mtu={mtu} config-interval=-1 pt=96 aggregate-mode={agg}",
                video_src = video_desc.pipeline_fragment,
                converter = video_desc.preferred_converter,
                caps = caps,
                enc_element = enc.gst_element(),
                enc_params = enc.encode_params(cfg),
                qbufs = qbufs,
                qtime = qtime,
                mtu = cfg.rtp_mtu,
                agg = cfg.aggregate_mode,
            )
        } else {
            format!(
                "{video_src} ! {converter} ! {caps}{enc_element} name=video_encoder {enc_params} ! \
                video/x-h264,profile=constrained-baseline ! h264parse config-interval=-1 disable-passthrough=true ! \
                video/x-h264,stream-format=byte-stream,alignment=au ! \
                queue max-size-buffers={qbufs} max-size-bytes=0 max-size-time={qtime} ! \
                rtph264pay mtu={mtu} config-interval=-1 pt=96 aggregate-mode={agg}",
                video_src = video_desc.pipeline_fragment,
                converter = video_desc.preferred_converter,
                caps = caps,
                enc_element = enc.gst_element(),
                enc_params = enc.encode_params(cfg),
                qbufs = qbufs,
                qtime = qtime,
                mtu = cfg.rtp_mtu,
                agg = cfg.aggregate_mode,
            )
        };

        let mut audio = if cfg.audio {
            let src = platform.build_audio_source(cfg);
            format!(
                "{} ! queue max-size-buffers=5 max-size-bytes=0 max-size-time=0 leaky=downstream ! \
                audioconvert ! audioresample ! opusenc inband-fec=true frame-size=10 audio-type=2051 bitrate={} ! rtpopuspay",
                src, audio_bitrate
            )
        } else {
            String::new()
        };

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

        let video_sink = format_multiudpsink(&cfg.client_host, 5004, udp_buf);
        let audio_branch = if cfg.audio {
            let audio_sink = format_multiudpsink(&cfg.client_host, 5006, 1048576);
            format!(" {} ! {}", audio, audio_sink)
        } else {
            String::new()
        };

        format!("{} ! {}{}", video, video_sink, audio_branch)
    }

    pub fn probe_capabilities() -> Capabilities {
        crate::config::probe_capabilities()
    }
}
