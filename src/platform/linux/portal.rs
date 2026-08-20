// src/platform/linux/portal.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use anyhow::{Result, anyhow};
use ashpd::desktop::screencast::{
    CursorMode, OpenPipeWireRemoteOptions, Screencast, SelectSourcesOptions, SourceType,
    StartCastOptions,
};
use ashpd::desktop::{CreateSessionOptions, PersistMode};
use std::os::fd::IntoRawFd;
use tokio::sync::broadcast::Sender;
use tracing::info;

use crate::ipc::OutboundMessage;
use crate::loc;

pub struct PortalCapture {
    pub node_id: u32,
    pub fd: i32,
    pub restore_token: Option<String>,
    _screencast_session: ashpd::desktop::Session<Screencast>,
}

pub async fn request_screencast(
    restore_token: Option<String>,
    tx: Option<Sender<OutboundMessage>>,
) -> Result<PortalCapture> {
    let clean_token = restore_token
        .as_ref()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());

    info!(
        "{}: restore_token={:?}",
        loc::MSG_PORTAL_REQUESTING,
        clean_token
    );

    let screencast_proxy = Screencast::new().await?;
    let screencast_session = screencast_proxy
        .create_session(CreateSessionOptions::default())
        .await?;

    // Crucial: set_sources(Monitor | Window) allows the portal to restore monitor selection without prompting
    screencast_proxy
        .select_sources(
            &screencast_session,
            SelectSourcesOptions::default()
                .set_cursor_mode(Some(CursorMode::Embedded))
                .set_sources(SourceType::Monitor | SourceType::Window)
                .set_multiple(false)
                .set_restore_token(clean_token.as_deref())
                .set_persist_mode(Some(PersistMode::ExplicitlyRevoked)),
        )
        .await?;

    if clean_token.is_none() {
        if let Some(ref tx) = tx {
            let _ = tx.send(OutboundMessage::WaitingForPortalApproval);
        }
    }

    let response = screencast_proxy
        .start(&screencast_session, None, StartCastOptions::default())
        .await?
        .response()?;

    let new_token = response
        .restore_token()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());

    let stream = response
        .streams()
        .first()
        .ok_or_else(|| anyhow!("portal returned no streams"))?;

    let node_id = stream.pipe_wire_node_id();
    info!(
        "{}: node_id={} | restore_token={:?}",
        loc::MSG_PORTAL_GRANTED,
        node_id,
        new_token
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
