use anyhow::{Result, anyhow};
use ashpd::desktop::screencast::{
    CursorMode, OpenPipeWireRemoteOptions, Screencast, SelectSourcesOptions, StartCastOptions,
};
use ashpd::desktop::{CreateSessionOptions, PersistMode};
use std::os::fd::IntoRawFd;
use tokio::sync::broadcast::Sender;
use tracing::info;

use crate::ipc::OutboundMessage;

/// Manages the lifetime of an active XDG Desktop Portal screencast session.
pub struct PortalCapture {
    pub node_id: u32,
    pub fd: i32,
    pub restore_token: Option<String>,
    _screencast_session: ashpd::desktop::Session<Screencast>,
}

/// Establishes screencast portal session without triggering RemoteDesktop input prompts.
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
                // ExplicitlyRevoked = persist until manually revoked (ashpd naming is inverted
                // vs intuition — this IS the "keep token alive" mode, NOT DoNot or Application)
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

    Ok(PortalCapture {
        node_id,
        fd: raw_fd,
        restore_token: new_token,
        _screencast_session: screencast_session,
    })
}