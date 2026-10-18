// src/stream.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use anyhow::{Result, anyhow};
use gst::prelude::*;
use gstreamer as gst;
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;
use tracing::info;

use crate::loc;
use crate::platform::PlatformBackend;

pub struct StreamManager {
    pipeline: Mutex<Option<gst::Pipeline>>,
    active_encoder: Mutex<String>,
    platform: Arc<dyn PlatformBackend>,
    outbound_tx: tokio::sync::broadcast::Sender<crate::ipc::OutboundMessage>,
    profiler_shutdown: Arc<AtomicBool>,
}

impl StreamManager {
    pub fn new(
        platform: Arc<dyn PlatformBackend>,
        outbound_tx: tokio::sync::broadcast::Sender<crate::ipc::OutboundMessage>,
    ) -> Self {
        Self {
            pipeline: Mutex::new(None),
            active_encoder: Mutex::new("none".into()),
            platform,
            outbound_tx,
            profiler_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_streaming(&self) -> bool {
        self.pipeline.lock().is_some()
    }

    pub fn start_pipeline(self: &Arc<Self>, pipeline_str: &str) -> Result<()> {
        let _ = self.stop();

        info!("{}", loc::MSG_PIPELINE_RESTARTING);
        self.profiler_shutdown.store(true, Ordering::Relaxed);

        let new_pipeline = gst::parse::launch(pipeline_str)?
            .dynamic_cast::<gst::Pipeline>()
            .map_err(|_| anyhow!("Failed to cast to pipeline"))?;

        let encoder_name = if let Some(enc) = new_pipeline.by_name("video_encoder") {
            enc.factory()
                .map(|f| f.name().to_string())
                .unwrap_or_else(|| "unknown".into())
        } else {
            "none".into()
        };

        self.platform.post_pipeline_start(&new_pipeline)?;
        new_pipeline.set_state(gst::State::Playing)?;

        *self.active_encoder.lock() = encoder_name;

        self.profiler_shutdown.store(false, Ordering::Relaxed);
        self.attach_detailed_probes(&new_pipeline, self.profiler_shutdown.clone());
        self.spawn_bus_listener(&new_pipeline);

        *self.pipeline.lock() = Some(new_pipeline);

        info!("{}: active_encoder={}", loc::MSG_STREAM_STARTED, self.active_encoder());
        Ok(())
    }

    /// Alias for backwards compatibility with existing IPC handlers
    #[allow(dead_code)]
    pub fn restart_pipeline(self: &Arc<Self>, pipeline_str: &str) -> Result<()> {
        self.start_pipeline(pipeline_str)
    }

    fn attach_detailed_probes(&self, pipeline: &gst::Pipeline, shutdown: Arc<AtomicBool>) {
        // In audio-only mode, no video encoder exists; skip video telemetry
        if pipeline.by_name("video_encoder").is_none() {
            return;
        }

        let cap_count = Arc::new(AtomicU32::new(0));
        let enc_in_count = Arc::new(AtomicU32::new(0));
        let enc_out_count = Arc::new(AtomicU32::new(0));
        let idr_count = Arc::new(AtomicU32::new(0));
        let p_count = Arc::new(AtomicU32::new(0));
        let bytes_sent = Arc::new(AtomicU64::new(0));

        let capture_src = pipeline
            .by_name("d3d11screencapturesrc0")
            .or_else(|| pipeline.by_name("pipewiresrc0"))
            .or_else(|| pipeline.by_name("gdi_src"))
            .or_else(|| pipeline.by_name("ximagesrc0"));

        if let Some(src) = capture_src {
            if let Some(src_pad) = src.src_pads().first() {
                let count = cap_count.clone();
                src_pad.add_probe(gst::PadProbeType::BUFFER, move |_, _| {
                    count.fetch_add(1, Ordering::Relaxed);
                    gst::PadProbeReturn::Ok
                });
            }
        }

        if let Some(enc) = pipeline.by_name("video_encoder") {
            if let Some(sink_pad) = enc.sink_pads().first() {
                let count = enc_in_count.clone();
                sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_, _| {
                    count.fetch_add(1, Ordering::Relaxed);
                    gst::PadProbeReturn::Ok
                });
            }

            if let Some(src_pad) = enc.src_pads().first() {
                let count = enc_out_count.clone();
                let idrs = idr_count.clone();
                let ps = p_count.clone();
                let bytes = bytes_sent.clone();

                src_pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {
                    count.fetch_add(1, Ordering::Relaxed);
                    if let Some(gst::PadProbeData::Buffer(ref buffer)) = probe_info.data {
                        bytes.fetch_add(buffer.size() as u64, Ordering::Relaxed);
                        if !buffer.flags().contains(gst::BufferFlags::DELTA_UNIT) {
                            idrs.fetch_add(1, Ordering::Relaxed);
                        } else {
                            ps.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    gst::PadProbeReturn::Ok
                });
            }
        }

        let c_cap = cap_count;
        let c_in = enc_in_count;
        let c_out = enc_out_count;
        let c_idr = idr_count;
        let c_p = p_count;
        let c_bytes = bytes_sent;

        std::thread::spawn(move || {
            let mut last_check = Instant::now();
            while !shutdown.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
                let elapsed = last_check.elapsed().as_secs_f64();
                last_check = Instant::now();

                let cap_fps = (c_cap.swap(0, Ordering::Relaxed) as f64 / elapsed).round() as u32;
                let in_fps = (c_in.swap(0, Ordering::Relaxed) as f64 / elapsed).round() as u32;
                let out_fps = (c_out.swap(0, Ordering::Relaxed) as f64 / elapsed).round() as u32;
                let idr_fps = (c_idr.swap(0, Ordering::Relaxed) as f64 / elapsed).round() as u32;
                let p_fps = (c_p.swap(0, Ordering::Relaxed) as f64 / elapsed).round() as u32;
                let mbps =
                    (c_bytes.swap(0, Ordering::Relaxed) as f64 * 8.0) / (1_000_000.0 * elapsed);

                info!(
                    "[STREAM TELEMETRY] Capture: {} FPS | EncIn: {} FPS | EncOut: {} FPS (IDR: {}, P: {}) | TX: {:.2} Mbps",
                    cap_fps, in_fps, out_fps, idr_fps, p_fps, mbps
                );
            }
        });
    }

    fn spawn_bus_listener(self: &Arc<Self>, pipeline: &gst::Pipeline) {
        let bus = match pipeline.bus() {
            Some(b) => b,
            None => return,
        };
        let tx = self.outbound_tx.clone();
        let manager = Arc::downgrade(self);

        std::thread::spawn(move || {
            while let Some(msg) = bus.timed_pop(gst::ClockTime::NONE) {
                use gst::MessageView;
                match msg.view() {
                    MessageView::Error(err) => {
                        let src = err
                            .src()
                            .map(|s| s.path_string().to_string())
                            .unwrap_or_else(|| "unknown".into());
                        let error_msg = format!(
                            "Error from element {}: {} | Debug context: {:?}",
                            src,
                            err.error(),
                            err.debug()
                        );
                        tracing::error!("{}", error_msg);
                        let _ = tx
                            .send(crate::ipc::OutboundMessage::StreamError { message: error_msg });

                        // Clean up the pipeline and reset manager to IDLE state
                        if let Some(mgr) = manager.upgrade() {
                            let _ = mgr.stop();
                        }
                        break;
                    }
                    MessageView::Warning(warn) => {
                        let src = warn
                            .src()
                            .map(|s| s.path_string().to_string())
                            .unwrap_or_else(|| "unknown".into());
                        tracing::warn!(
                            "Warning from element {}: {} ({:?})",
                            src,
                            warn.error(),
                            warn.debug()
                        );
                    }
                    MessageView::Eos(_) => {
                        tracing::info!("End of stream reached");
                        if let Some(mgr) = manager.upgrade() {
                            let _ = mgr.stop();
                        }
                        break;
                    }
                    _ => {}
                }
            }
        });
    }

    pub fn active_encoder(&self) -> String {
        self.active_encoder.lock().clone()
    }

    pub fn update_bitrate(&self, bitrate: u32) -> Result<()> {
        let pipeline_guard = self.pipeline.lock();
        if let Some(ref pipeline) = *pipeline_guard {
            if let Some(encoder) = pipeline.by_name("video_encoder") {
                encoder.set_property("bitrate", bitrate);
                info!("{}: {} kbps", loc::MSG_BITRATE_UPDATED, bitrate);
            }
        }
        Ok(())
    }

    pub fn force_keyframe(&self) -> Result<()> {
        let pipeline_guard = self.pipeline.lock();
        if let Some(ref pipeline) = *pipeline_guard {
            if let Some(encoder) = pipeline.by_name("video_encoder") {
                let s = gst::Structure::builder("GstForceKeyUnit")
                    .field("all-headers", true)
                    .field("count", 1i32)
                    .build();

                if let Some(pad) = encoder.sink_pads().first() {
                    let event = gst::event::CustomUpstream::new(s);
                    if pad.send_event(event) {
                        info!("{}", loc::MSG_KEYFRAME_SENT);
                    } else {
                        tracing::warn!("{}", loc::MSG_KEYFRAME_REFUSED);
                    }
                } else {
                    let event = gst::event::CustomUpstream::new(s);
                    encoder.send_event(event);
                }
            }
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.profiler_shutdown.store(true, Ordering::Relaxed);
        let mut pipeline_guard = self.pipeline.lock();
        if let Some(pipeline) = pipeline_guard.take() {
            self.platform.pre_pipeline_stop();
            let _ = pipeline.set_state(gst::State::Null);
            *self.active_encoder.lock() = "none".into();
            info!("{}", loc::MSG_STREAM_STOPPED);
        }
        Ok(())
    }
}