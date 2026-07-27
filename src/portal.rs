use anyhow::{Result, anyhow};
use ashpd::desktop::remote_desktop::{
    DeviceType, NotifyPointerAxisOptions, NotifyPointerButtonOptions,
    NotifyPointerMotionAbsoluteOptions, NotifyPointerMotionOptions, RemoteDesktop,
    SelectDevicesOptions,
};
use ashpd::desktop::screencast::{
    CursorMode, OpenPipeWireRemoteOptions, Screencast, SelectSourcesOptions, StartCastOptions,
};
use ashpd::desktop::{CreateSessionOptions, PersistMode};
use std::os::fd::IntoRawFd;
use tokio::sync::broadcast::Sender;
use tracing::{debug, info, warn};

use crate::input::{MouseGesture, MouseInput, mouse_button_to_linux_code};
use crate::ipc::OutboundMessage;

/// Manages the lifetime of an active XDG Desktop Portal screencast and remote desktop input session.
pub struct PortalCapture {
    pub node_id: u32,
    pub fd: i32,
    pub restore_token: Option<String>,
    _screencast_session: ashpd::desktop::Session<Screencast>,
    remote_desktop: Option<RemoteDesktop>,
    rd_session: Option<ashpd::desktop::Session<RemoteDesktop>>,
}

impl PortalCapture {
    pub async fn handle_mouse_input(&self, input: &MouseInput) {
        let (rd, session) = match (&self.remote_desktop, &self.rd_session) {
            (Some(rd), Some(session)) => (rd, session),
            _ => {
                warn!("RemoteDesktop portal session not active; ignoring input");
                return;
            }
        };

        debug!("Portal processing mouse input: {:?}", input);

        match input {
            MouseInput::Move { x, y, absolute } => {
                if *absolute {
                    let opts = NotifyPointerMotionAbsoluteOptions::default();
                    if let Err(e) = rd
                        .notify_pointer_motion_absolute(session, self.node_id, *x, *y, opts)
                        .await
                    {
                        debug!("Portal notify_pointer_motion_absolute error: {}", e);
                    }
                } else {
                    let opts = NotifyPointerMotionOptions::default();
                    if let Err(e) = rd.notify_pointer_motion(session, *x, *y, opts).await {
                        debug!("Portal notify_pointer_motion error: {}", e);
                    }
                }
            }
            MouseInput::ButtonDown { button } => {
                let code = mouse_button_to_linux_code(button);
                let opts = NotifyPointerButtonOptions::default();
                let _ = rd
                    .notify_pointer_button(
                        session,
                        code,
                        ashpd::desktop::remote_desktop::KeyState::Pressed,
                        opts,
                    )
                    .await;
            }
            MouseInput::ButtonUp { button } => {
                let code = mouse_button_to_linux_code(button);
                let opts = NotifyPointerButtonOptions::default();
                let _ = rd
                    .notify_pointer_button(
                        session,
                        code,
                        ashpd::desktop::remote_desktop::KeyState::Released,
                        opts,
                    )
                    .await;
            }
            MouseInput::Click { button } => {
                let code = mouse_button_to_linux_code(button);
                let opts1 = NotifyPointerButtonOptions::default();
                let opts2 = NotifyPointerButtonOptions::default();
                let _ = rd
                    .notify_pointer_button(
                        session,
                        code,
                        ashpd::desktop::remote_desktop::KeyState::Pressed,
                        opts1,
                    )
                    .await;
                let _ = rd
                    .notify_pointer_button(
                        session,
                        code,
                        ashpd::desktop::remote_desktop::KeyState::Released,
                        opts2,
                    )
                    .await;
            }
            MouseInput::DoubleClick { button } => {
                let code = mouse_button_to_linux_code(button);
                for _ in 0..2 {
                    let opts1 = NotifyPointerButtonOptions::default();
                    let opts2 = NotifyPointerButtonOptions::default();
                    let _ = rd
                        .notify_pointer_button(
                            session,
                            code,
                            ashpd::desktop::remote_desktop::KeyState::Pressed,
                            opts1,
                        )
                        .await;
                    let _ = rd
                        .notify_pointer_button(
                            session,
                            code,
                            ashpd::desktop::remote_desktop::KeyState::Released,
                            opts2,
                        )
                        .await;
                }
            }
            MouseInput::Scroll { delta_x, delta_y } => {
                if *delta_y != 0.0 {
                    let opts = NotifyPointerAxisOptions::default();
                    let _ = rd.notify_pointer_axis(session, 0.0, *delta_y, opts).await;
                }
                if *delta_x != 0.0 {
                    let opts = NotifyPointerAxisOptions::default();
                    let _ = rd.notify_pointer_axis(session, *delta_x, 0.0, opts).await;
                }
            }
            MouseInput::Gesture(gesture) => match gesture {
                MouseGesture::Pinch { scale } => {
                    let opts = NotifyPointerAxisOptions::default();
                    let _ = rd
                        .notify_pointer_axis(
                            session,
                            0.0,
                            if *scale > 1.0 { 1.0 } else { -1.0 },
                            opts,
                        )
                        .await;
                }
                MouseGesture::TwoFingerScroll { dx, dy } => {
                    let opts = NotifyPointerAxisOptions::default();
                    let _ = rd.notify_pointer_axis(session, *dx, *dy, opts).await;
                }
                MouseGesture::Swipe { dx, dy } => {
                    let opts = NotifyPointerMotionOptions::default();
                    let _ = rd.notify_pointer_motion(session, *dx, *dy, opts).await;
                }
                MouseGesture::Rotate { angle } => {
                    let opts = NotifyPointerAxisOptions::default();
                    let _ = rd
                        .notify_pointer_axis(
                            session,
                            if *angle > 0.0 { 1.0 } else { -1.0 },
                            0.0,
                            opts,
                        )
                        .await;
                }
            },
        }
    }
}

/// Establishes screencast and RemoteDesktop portal sessions.
pub async fn request_screencast(
    restore_token: Option<String>,
    tx: Option<Sender<OutboundMessage>>,
) -> Result<PortalCapture> {
    info!(
        "requesting screen capture via XDG portal... (restore_token: {:?})",
        restore_token
    );

    let screencast_proxy = Screencast::new().await?;
    let screencast_session = screencast_proxy
        .create_session(CreateSessionOptions::default())
        .await?;

    screencast_proxy
        .select_sources(
            &screencast_session,
            SelectSourcesOptions::default()
                .set_cursor_mode(Some(CursorMode::Embedded))
                .set_restore_token(restore_token.as_deref())
                .set_persist_mode(Some(PersistMode::ExplicitlyRevoked)),
        )
        .await?;

    if restore_token.is_none() {
        if let Some(ref tx) = tx {
            let _ = tx.send(OutboundMessage::WaitingForPortalApproval);
        }
    }

    let response = screencast_proxy
        .start(&screencast_session, None, StartCastOptions::default())
        .await?
        .response()?;
    let new_token = response.restore_token().map(|t| t.to_string());

    let stream = response
        .streams()
        .first()
        .ok_or_else(|| anyhow!("portal returned no streams"))?;

    let node_id = stream.pipe_wire_node_id();
    info!(
        "portal granted stream: node_id={} | restore_token: {:?}",
        node_id, new_token
    );

    let fd = screencast_proxy
        .open_pipe_wire_remote(&screencast_session, OpenPipeWireRemoteOptions::default())
        .await?;
    let raw_fd = fd.into_raw_fd();

    // Initialize and start RemoteDesktop session for input permission
    let (rd_proxy, rd_session) = match RemoteDesktop::new().await {
        Ok(rd) => match rd.create_session(CreateSessionOptions::default()).await {
            Ok(sess) => {
                let opts = SelectDevicesOptions::default()
                    .set_devices(ashpd::enumflags2::BitFlags::from(DeviceType::Pointer));
                let _ = rd.select_devices(&sess, opts).await;
                match rd
                    .start(
                        &sess,
                        None,
                        ashpd::desktop::remote_desktop::StartOptions::default(),
                    )
                    .await
                {
                    Ok(resp) => {
                        if let Ok(_) = resp.response() {
                            info!(
                                "RemoteDesktop portal session started and authorized successfully"
                            );
                        }
                    }
                    Err(e) => warn!("RemoteDesktop start error: {}", e),
                }
                (Some(rd), Some(sess))
            }
            Err(e) => {
                warn!("RemoteDesktop create_session error: {}", e);
                (None, None)
            }
        },
        Err(e) => {
            warn!("RemoteDesktop proxy unavailable: {}", e);
            (None, None)
        }
    };

    Ok(PortalCapture {
        node_id,
        fd: raw_fd,
        restore_token: new_token,
        _screencast_session: screencast_session,
        remote_desktop: rd_proxy,
        rd_session,
    })
}
