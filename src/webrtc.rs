use anyhow::{Result, anyhow};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_sdp as gst_sdp;
use gstreamer_webrtc as gst_webrtc;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::info;

use crate::ipc::OutboundMessage;

pub struct StreamManager {
    pipeline: Mutex<gst::Pipeline>,
    webrtcbin: Mutex<Option<gst::Element>>,
    sender: tokio::sync::broadcast::Sender<OutboundMessage>,
    active_encoder: Mutex<String>,
    #[cfg(target_os = "windows")]
    gdi_capture_running: Mutex<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>>,
}

impl StreamManager {
    pub fn new(
        pipeline_str: &str,
        sender: tokio::sync::broadcast::Sender<OutboundMessage>,
    ) -> Result<Self> {
        gst::init()?;

        let pipeline = gst::parse::launch(pipeline_str)?
            .dynamic_cast::<gst::Pipeline>()
            .map_err(|_| anyhow!("Failed to cast to pipeline"))?;

        let webrtcbin = pipeline.by_name("sendrecv");

        let active_encoder = if let Some(enc) = pipeline.by_name("video_encoder") {
            enc.factory()
                .map(|f| f.name().to_string())
                .unwrap_or_else(|| "unknown".into())
        } else {
            "none".into()
        };

        #[cfg(target_os = "windows")]
        let mut gdi_capture_running = None;
        #[cfg(target_os = "windows")]
        if let Some(src) = pipeline.by_name("gdi_src") {
            let appsrc = src.downcast::<gstreamer_app::AppSrc>().unwrap();
            gdi_capture_running = Some(crate::gdi_capture::start_gdi_capture(appsrc));
        }

        Ok(Self {
            pipeline: Mutex::new(pipeline),
            webrtcbin: Mutex::new(webrtcbin),
            sender,
            active_encoder: Mutex::new(active_encoder),
            #[cfg(target_os = "windows")]
            gdi_capture_running: Mutex::new(gdi_capture_running),
        })
    }

    pub fn active_encoder(&self) -> String {
        self.active_encoder.lock().clone()
    }

    pub fn start(self: &Arc<Self>) -> Result<()> {
        let sender = self.sender.clone();
        let pipeline = self.pipeline.lock();

        if let Some(webrtcbin) = self.webrtcbin.lock().as_ref() {
            // Broadcast local ICE candidates via the signaling channel to negotiate connectivity with the remote peer.
            webrtcbin.connect("on-ice-candidate", false, move |values| {
                if let (Some(mline_index), Some(candidate)) =
                    (values[1].get::<u32>().ok(), values[2].get::<String>().ok())
                {
                    let _ = sender.send(OutboundMessage::LocalIceCandidate {
                        candidate,
                        mid: None,
                        mline_index: Some(mline_index),
                    });
                }
                None
            });

            // Monitor connection state to auto-stop when client disappears, preventing zombie engines from blocking the server.
            let self_clone = Arc::clone(self);
            webrtcbin.connect("on-ice-connection-state-changed", false, move |values| {
                if let Some(state) = values[0].get::<String>().ok() {
                    info!("WebRTC ICE connection state changed: {}", state);
                    if state == "failed" || state == "closed" || state == "disconnected" {
                        info!("Client disconnected or connection failed. Stopping pipeline to release resources.");
                        let manager = Arc::clone(&self_clone);
                        tokio::spawn(async move {
                            let _ = manager.stop();
                        });
                    }
                }
                None
            });
        }

        pipeline.set_state(gst::State::Playing)?;
        Ok(())
    }

    /// Reconstructs the GStreamer pipeline in-place without restarting the host process.
    /// Preserving the active process context ensures that system-level capture sessions (such as
    /// XDG Desktop Portals or screen-capture tokens) remain valid, preventing redundant user authorization prompts.
    pub fn restart_pipeline(&self, pipeline_str: &str) -> Result<()> {
        info!("restarting pipeline in-place...");
        let mut pipeline = self.pipeline.lock();

        let _ = pipeline.set_state(gst::State::Null);

        #[cfg(target_os = "windows")]
        if let Some(r) = self.gdi_capture_running.lock().take() {
            r.store(false, std::sync::atomic::Ordering::SeqCst);
        }

        let new_pipeline = gst::parse::launch(pipeline_str)?
            .dynamic_cast::<gst::Pipeline>()
            .map_err(|_| anyhow!("Failed to cast to pipeline"))?;

        if let Some(enc) = new_pipeline.by_name("video_encoder") {
            let name = enc
                .factory()
                .map(|f| f.name().to_string())
                .unwrap_or("unknown".into());
            *self.active_encoder.lock() = name;
        }

        // Update the WebRTC bin reference; this resolves to None if running in raw RTP streaming mode.
        *self.webrtcbin.lock() = new_pipeline.by_name("sendrecv");

        new_pipeline.set_state(gst::State::Playing)?;
        *pipeline = new_pipeline;

        #[cfg(target_os = "windows")]
        if let Some(src) = pipeline.by_name("gdi_src") {
            let appsrc = src.downcast::<gstreamer_app::AppSrc>().unwrap();
            *self.gdi_capture_running.lock() = Some(crate::gdi_capture::start_gdi_capture(appsrc));
        }

        info!("pipeline restarted successfully");
        Ok(())
    }

    pub fn generate_offer(&self) -> Result<()> {
        let webrtcbin_guard = self.webrtcbin.lock();
        let webrtcbin = webrtcbin_guard
            .as_ref()
            .ok_or_else(|| anyhow!("Not in WebRTC mode"))?;
        info!("generating offer on demand");
        let sender_inner = self.sender.clone();
        let webrtc_ref = webrtcbin.clone();

        let promise = gst::Promise::with_change_func(move |reply| {
            if let Ok(Some(reply)) = reply {
                if let Some(offer) = reply
                    .get::<gst_webrtc::WebRTCSessionDescription>("offer")
                    .ok()
                {
                    // Commit the generated offer to the peer connection's local state machine
                    // before transmitting it, preparing the bin to ingest remote ICE candidates.
                    webrtc_ref.emit_by_name::<()>(
                        "set-local-description",
                        &[&offer, &None::<gst::Promise>],
                    );

                    if let Some(sdp_text) = offer.sdp().as_text().ok() {
                        let _ = sender_inner.send(OutboundMessage::LocalSdpGenerated {
                            sdp: sdp_text,
                            sdp_type: "offer".to_string(),
                        });
                    }
                }
            }
        });

        webrtcbin.emit_by_name::<()>("create-offer", &[&None::<gst::Structure>, &promise]);
        Ok(())
    }

    pub fn handle_remote_sdp(&self, sdp: &str, type_: &str) -> Result<()> {
        let webrtcbin_guard = self.webrtcbin.lock();
        let webrtcbin = webrtcbin_guard
            .as_ref()
            .ok_or_else(|| anyhow!("Not in WebRTC mode"))?;
        let sdp_type = match type_ {
            "offer" => gst_webrtc::WebRTCSDPType::Offer,
            "answer" => gst_webrtc::WebRTCSDPType::Answer,
            _ => return Err(anyhow!("invalid sdp type")),
        };

        let res = gst_webrtc::WebRTCSessionDescription::new(
            sdp_type,
            gst_sdp::SDPMessage::parse_buffer(sdp.as_bytes())?,
        );
        webrtcbin.emit_by_name::<()>("set-remote-description", &[&res, &None::<gst::Promise>]);
        Ok(())
    }

    pub fn add_ice_candidate(
        &self,
        candidate: &str,
        _mid: Option<String>,
        mline_index: Option<u32>,
    ) -> Result<()> {
        let webrtcbin_guard = self.webrtcbin.lock();
        let webrtcbin = webrtcbin_guard
            .as_ref()
            .ok_or_else(|| anyhow!("Not in WebRTC mode"))?;
        webrtcbin.emit_by_name::<()>(
            "add-ice-candidate",
            &[&(mline_index.unwrap_or(0)), &candidate],
        );
        Ok(())
    }

    pub fn update_bitrate(&self, bitrate: u32) -> Result<()> {
        let pipeline = self.pipeline.lock();
        if let Some(encoder) = pipeline.by_name("video_encoder") {
            encoder.set_property("bitrate", bitrate);
            info!("dynamically updated encoder bitrate to {} kbps", bitrate);
        } else {
            tracing::warn!("video_encoder element not found in pipeline");
        }
        Ok(())
    }

    pub fn force_keyframe(&self) -> Result<()> {
        let pipeline = self.pipeline.lock();
        if let Some(encoder) = pipeline.by_name("video_encoder") {
            info!("forcing keyframe on encoder...");
            let s = gst::Structure::builder("GstForceKeyUnit")
                .field("all-headers", true)
                .build();
            let event = gst::event::CustomDownstream::new(s);
            encoder.send_event(event);
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let pipeline = self.pipeline.lock();
        let _ = pipeline.set_state(gst::State::Null);
        #[cfg(target_os = "windows")]
        if let Some(r) = self.gdi_capture_running.lock().take() {
            r.store(false, std::sync::atomic::Ordering::SeqCst);
        }
        Ok(())
    }
}