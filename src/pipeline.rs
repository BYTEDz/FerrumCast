// src/pipeline.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use crate::config::{Capabilities, StreamConfig};
use crate::platform::PlatformBackend;
use tracing::info;

pub mod encoders;
pub mod generic;
pub mod transport;

pub struct PipelineBuilder;

impl PipelineBuilder {
    pub fn build_pipeline(
        cfg: &StreamConfig,
        enc: &dyn encoders::VideoEncoder,
        platform: &dyn PlatformBackend,
    ) -> String {
        if cfg.audio_only {
            info!(
                "Building Audio-Only stream pipeline (clients={}, audio_bitrate={}bps)",
                cfg.client_host,
                cfg.audio_bitrate * 1000
            );
            let audio_src = platform.build_audio_source(cfg);
            return transport::build_standalone_audio_pipeline(&audio_src, cfg);
        }

        let is_hw = enc.is_hardware();
        let video_desc = platform.build_video_source(cfg, enc);

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

        // Construct the video encoder element with explicit naming for IPC dynamic bitrate & keyframe controls
        let encoded_fragment = format!(
            "{video_src} ! {converter} ! {caps}{enc_element} name=video_encoder {enc_params}",
            video_src = video_desc.pipeline_fragment,
            converter = video_desc.preferred_converter,
            caps = caps,
            enc_element = enc.gst_element(),
            enc_params = enc.encode_params(cfg),
        );

        let video_branch = transport::build_video_transport(&encoded_fragment, enc, cfg);

        let audio_branch = if cfg.audio {
            let audio_src = platform.build_audio_source(cfg);
            transport::build_audio_branch(&audio_src, cfg)
        } else {
            String::new()
        };

        format!("{}{}", video_branch, audio_branch)
    }

    pub fn probe_capabilities() -> Capabilities {
        crate::config::probe_capabilities()
    }
}