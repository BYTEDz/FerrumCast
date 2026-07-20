use anyhow::{Result, anyhow};
use gst::prelude::*;
use gstreamer as gst;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::info;

pub struct StreamManager {
    pipeline: Mutex<gst::Pipeline>,
    active_encoder: Mutex<String>,
    #[cfg(target_os = "windows")]
    gdi_capture_running: Mutex<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>>,
    outbound_tx: tokio::sync::broadcast::Sender<crate::ipc::OutboundMessage>,
}

impl StreamManager {
    pub fn new(
        pipeline_str: &str,
        outbound_tx: tokio::sync::broadcast::Sender<crate::ipc::OutboundMessage>,
    ) -> Result<Self> {
        let pipeline = gst::parse::launch(pipeline_str)?
            .dynamic_cast::<gst::Pipeline>()
            .map_err(|_| anyhow!("Failed to cast to pipeline"))?;

        let active_encoder = if let Some(enc) = pipeline.by_name("video_encoder") {
            enc.factory().map(|f| f.name().to_string()).unwrap_or_else(|| "unknown".into())
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

        let manager = Self {
            pipeline: Mutex::new(pipeline),
            active_encoder: Mutex::new(active_encoder),
            #[cfg(target_os = "windows")]
            gdi_capture_running: Mutex::new(gdi_capture_running),
            outbound_tx,
        };

        manager.spawn_bus_listener(&manager.pipeline.lock());

        Ok(manager)
    }

    fn spawn_bus_listener(&self, pipeline: &gst::Pipeline) {
        let bus = match pipeline.bus() {
            Some(b) => b,
            None => return,
        };
        let tx = self.outbound_tx.clone();
        std::thread::spawn(move || {
            while let Some(msg) = bus.timed_pop(gst::ClockTime::NONE) {
                use gst::MessageView;
                match msg.view() {
                    MessageView::Error(err) => {
                        let src = err.src().map(|s| s.path_string().to_string()).unwrap_or_else(|| "unknown".into());
                        let error_msg = format!(
                            "Error from element {}: {} | Debug context: {:?}",
                            src,
                            err.error(),
                            err.debug()
                        );
                        tracing::error!("{}", error_msg);
                        let _ = tx.send(crate::ipc::OutboundMessage::StreamError {
                            message: error_msg,
                        });
                        break;
                    }
                    MessageView::Warning(warn) => {
                        let src = warn.src().map(|s| s.path_string().to_string()).unwrap_or_else(|| "unknown".into());
                        tracing::warn!("Warning from element {}: {} ({:?})", src, warn.error(), warn.debug());
                    }
                    MessageView::Eos(_) => {
                        tracing::info!("End of stream reached");
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

    pub fn start(self: &Arc<Self>) -> Result<()> {
        let pipeline = self.pipeline.lock();
        pipeline.set_state(gst::State::Playing)?;
        Ok(())
    }

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
            *self.active_encoder.lock() = enc.factory().map(|f| f.name().to_string()).unwrap_or("unknown".into());
        }

        new_pipeline.set_state(gst::State::Playing)?;
        *pipeline = new_pipeline;

        #[cfg(target_os = "windows")]
        if let Some(src) = pipeline.by_name("gdi_src") {
            let appsrc = src.downcast::<gstreamer_app::AppSrc>().unwrap();
            *self.gdi_capture_running.lock() = Some(crate::gdi_capture::start_gdi_capture(appsrc));
        }

        self.spawn_bus_listener(&pipeline);

        info!("pipeline restarted successfully");
        Ok(())
    }

    pub fn update_bitrate(&self, bitrate: u32) -> Result<()> {
        let pipeline = self.pipeline.lock();
        if let Some(encoder) = pipeline.by_name("video_encoder") {
            encoder.set_property("bitrate", bitrate);
            info!("dynamically updated encoder bitrate to {} kbps", bitrate);
        }
        Ok(())
    }

    pub fn force_keyframe(&self) -> Result<()> {
        let pipeline = self.pipeline.lock();
        if let Some(encoder) = pipeline.by_name("video_encoder") {
            let s = gst::Structure::builder("GstForceKeyUnit")
                .field("all-headers", true)
                .field("count", 1i32)
                .build();

            // Upstream events must be sent directly to the encoder's sink pad
            if let Some(pad) = encoder.sink_pads().first() {
                let event = gst::event::CustomUpstream::new(s);
                if pad.send_event(event) {
                    info!("Sent upstream ForceKeyUnit event to encoder sink pad");
                } else {
                    tracing::warn!("Encoder sink pad refused the keyframe event");
                }
            } else {
                let event = gst::event::CustomUpstream::new(s);
                encoder.send_event(event);
            }
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