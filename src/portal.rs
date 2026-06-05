use anyhow::{Result, anyhow};
use ashpd::desktop::screencast::Screencast;
use std::os::fd::IntoRawFd;
use tokio::sync::broadcast::Sender;
use tracing::info;

use crate::ipc::OutboundMessage;

/// Manages the lifetime of an active XDG Desktop Portal screencast session.
///
/// Retaining an instance of this struct preserves the underlying D-Bus session and the associated
/// PipeWire file descriptor, maintaining stream authorization via RAII.
pub struct PortalCapture {
    pub node_id: u32,
    pub fd: i32,
    pub restore_token: Option<String>,
    _session: ashpd::desktop::Session<ashpd::desktop::screencast::Screencast>,
}

/// Establishes a screen capture session via the XDG Desktop Portal.
///
/// If a valid `restore_token` is provided, the portal can bypass the user-facing permission
/// dialog and immediately resume streaming. If the token is missing, expired, or invalid,
/// the system-native authorization dialog will be displayed to the user.
pub async fn request_screencast(
    restore_token: Option<String>,
    tx: Option<Sender<OutboundMessage>>,
) -> Result<PortalCapture> {
    info!(
        "requesting screen capture via XDG portal... (restore_token: {:?})",
        restore_token
    );

    let proxy = Screencast::new().await?;
    let session = proxy.create_session(ashpd::desktop::CreateSessionOptions::default()).await?;

    proxy
        .select_sources(
            &session,
            ashpd::desktop::screencast::SelectSourcesOptions::default(),
        )
        .await?;

    if restore_token.is_none() {
        if let Some(ref tx) = tx {
            let _ = tx.send(OutboundMessage::WaitingForPortalApproval);
        }
    }

    // Start the screencast stream. This triggers a system-native authorization dialog 
    // if a valid restore token is not active or available.
    let response = proxy.start(&session, None, ashpd::desktop::screencast::StartCastOptions::default()).await?.response()?;
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

    let fd = proxy.open_pipe_wire_remote(&session, ashpd::desktop::screencast::OpenPipeWireRemoteOptions::default()).await?;
    let raw_fd = fd.into_raw_fd();

    Ok(PortalCapture {
        node_id,
        fd: raw_fd,
        restore_token: new_token,
        _session: session,
    })
}