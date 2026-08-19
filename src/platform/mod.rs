// src/platform/mod.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use anyhow::Result;
use gstreamer as gst;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;

use crate::config::StreamConfig;
use crate::input::MouseInput;
use crate::ipc::OutboundMessage;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DisplayServer {
    Wayland,
    X11,
    WindowsDesktop,
    WindowsHeadlessOrVm,
    Unknown,
}

pub struct VideoSourceDescriptor {
    pub pipeline_fragment: String,
    pub preferred_memory_feature: Option<&'static str>,
    pub preferred_converter: String,
    pub raw_caps_filter: Option<String>,
}

pub trait PlatformBackend: Send + Sync {
    #[allow(dead_code)]
    fn detect_display_server(&self) -> DisplayServer;
    fn build_video_source(
        &self,
        cfg: &StreamConfig,
        is_hardware_encoder: bool,
    ) -> VideoSourceDescriptor;
    fn build_audio_source(&self, cfg: &StreamConfig) -> String;
    fn handle_mouse_input(&self, input: &MouseInput);
    fn post_pipeline_start(&self, pipeline: &gst::Pipeline) -> Result<()>;
    fn pre_pipeline_stop(&self);
}

#[cfg(target_os = "linux")]
pub async fn create_platform_backend(
    initial_token: Option<String>,
    outbound_tx: Sender<OutboundMessage>,
    audio_only: bool,
) -> Result<Arc<dyn PlatformBackend>> {
    let backend = linux::LinuxBackend::new(initial_token, outbound_tx, audio_only).await?;
    Ok(Arc::new(backend))
}

#[cfg(target_os = "windows")]
pub async fn create_platform_backend(
    _initial_token: Option<String>,
    _outbound_tx: Sender<OutboundMessage>,
    _audio_only: bool,
) -> Result<Arc<dyn PlatformBackend>> {
    let backend = windows::WindowsBackend::new();
    Ok(Arc::new(backend))
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub async fn create_platform_backend(
    _initial_token: Option<String>,
    _outbound_tx: Sender<OutboundMessage>,
    _audio_only: bool,
) -> Result<Arc<dyn PlatformBackend>> {
    compile_error!("Unsupported target platform");
}
