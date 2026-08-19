// src/loc.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

#![allow(dead_code)]

pub const MSG_MOUSE_INPUT_RECEIVED: &str = "mouse_input_received";
pub const MSG_MOUSE_INPUT_SUCCESS: &str = "mouse_input_success";
pub const MSG_MOUSE_INPUT_FAILED: &str = "mouse_input_failed";
pub const MSG_PORTAL_NOT_AVAILABLE: &str = "portal_remote_desktop_not_available";
pub const MSG_NO_ACTIVE_PORTAL: &str = "no_active_linux_portal_instance";
pub const MSG_SEND_INPUT_FAILED: &str = "send_input_failed";
pub const MSG_EXECUTING_WIN32_MOUSE: &str = "executing_win32_mouse_input";
pub const MSG_IPC_CLIENT_CONNECTED: &str = "ipc_client_connected";
pub const MSG_IPC_CLIENT_DISCONNECTED: &str = "ipc_client_disconnected";
pub const MSG_IPC_DESERIALIZATION_FAILED: &str = "ipc_json_deserialization_failed";
pub const MSG_GDI_CAPTURE_ACTIVE: &str = "gdi_capture_active";
pub const MSG_GDI_GET_DC_FAILED: &str = "gdi_get_dc_failed";
pub const MSG_PORTAL_REQUESTING: &str = "portal_requesting_screencast";
pub const MSG_PORTAL_GRANTED: &str = "portal_screencast_granted";
pub const MSG_PORTAL_FAILED: &str = "portal_screencast_failed";
pub const MSG_SAVED_TOKEN_PERSISTING: &str = "portal_token_persisting";
pub const MSG_SAVED_TOKEN_INVALID: &str = "portal_saved_token_invalid_retrying";
pub const MSG_BACKEND_INITIALIZED: &str = "platform_backend_initialized";
pub const MSG_PIPELINE_RESTARTING: &str = "pipeline_restarting";
pub const MSG_PIPELINE_RESTARTED: &str = "pipeline_restarted";
pub const MSG_BITRATE_UPDATED: &str = "encoder_bitrate_updated";
pub const MSG_KEYFRAME_SENT: &str = "encoder_force_keyframe_sent";
pub const MSG_KEYFRAME_REFUSED: &str = "encoder_force_keyframe_refused";
pub const MSG_MONITOR_SWITCHING: &str = "monitor_switching";
pub const MSG_STARTING_ENGINE: &str = "engine_starting";
pub const MSG_ENGINE_READY: &str = "engine_ready";
pub const MSG_ENGINE_SHUTDOWN: &str = "engine_shutdown";
pub const MSG_ENV_DETECTED_WAYLAND: &str = "display_env_detected_wayland";
pub const MSG_ENV_DETECTED_X11: &str = "display_env_detected_x11";
pub const MSG_ENV_DETECTED_HEADLESS: &str = "display_env_detected_headless";
pub const MSG_ENV_DETECTED_D3D11: &str = "display_env_detected_d3d11";
pub const MSG_ENV_DETECTED_GDI_FALLBACK: &str = "display_env_detected_gdi_fallback";
