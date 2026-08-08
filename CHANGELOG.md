# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-08-08

- feat: add automated release script to handle version bumping and changelog generation
- refactor: sync Cargo.toml versions with release tags and migrate to custom Python-based release note generation
- feat: implement CLI version flag and update dynamic build versioning logic
- feat: implement automatic portal token refresh by retrying with a fresh prompt if the saved token is stale
- refactor: reorder automatic hardware encoder selection priority and group by codec support
- fix: update regex in DLL dependency finder to correctly capture versioned filenames
- feat: enhance Windows DLL loading by enforcing strict isolation and setting plugin paths
- chore: update license from AGPL-3.0 to PolyForm Noncommercial License 1.0.0 and update README accordingly
- chore: optimize CI build pipeline, update GStreamer installation, and add H.265 support for multiple hardware encoders
- refactor: flatten GStreamer deployment layout on Windows to improve plugin discovery and dependency resolution
- feat: verify GStreamer encoder state initialization and redirect tracing output to stderr
- feat: replace broadcast-based udpsink with configurable multiudpsink for client support
- feat: upgrade audio pipeline to multicast broadcast and refine video pipeline resolution and framerate handling
- feat: add audio_bitrate configuration, improve CLI flag parsing, and bypass XDG portal for audio-only streams
- feat: add audio-only streaming mode to configuration and pipeline execution
- refactor: simplify Windows plugin path handling and improve environment variable setup
- refactor: improve Windows executable path handling and simplify videorate configuration
- refactor: enhance GStreamer distribution process with improved plugin handling and dependency resolution
- refactor: enhance GStreamer plugin directory handling and improve environment variable setup
- feat: add monitor switching functionality and enhance video source handling
- refactor: update Linux token file path handling and clean up portal input management
- refactor: enhance IPC message handling and improve Windows mouse input logging
- refactor: improve mouse input handling on Windows and add error logging for SendInput failures
- refactor: compact OutboundMessage enum definitions and handle broadcast lag in IPC write loop
- fix: constrain portal capture to Linux and update Windows mouse input event constants
- chore: add Win32_UI_Input_KeyboardAndMouse feature to windows dependency
- refactor: improve portal mouse input handling, add Windows sub-pixel movement accumulation, and apply code formatting
- feat: add Linux XDG RemoteDesktop portal support for mouse input handling
- feat: enhance build workflow to support versioned artifact naming and improve artifact upload process
- feat: implement dynamic versioning and enhance release workflow with changelog configuration
- refactor: optimize D3D11 pipeline by integrating GPU-accelerated scaling and color conversion into the capture stage
- refactor: remove ResolvedEncoder and consolidate encoder logic into a new encoders module
- feat: enhance video source configuration and add outbound message handling in StreamManager
- docs: Enhance README with project structure and SRTP details
- chore: Update Rust dependencies in Cargo.lock
- refactor: remove WebRTC support, consolidate pipeline logic into a new stream module, and enable zero-copy DMA-BUF streaming on Linux
- Revert to working state at fdeecb3c
- deps: Update Rust dependencies
- feat: add SRTP encryption support for RTP output streams via optional master key configuration
- feat: add ICE connection state monitoring to automatically stop pipeline on client disconnect
- fix(windows): enable process-level DPI awareness on startup
- chore: update windows runner to 2025 and bump action versions to support windows platform flexibility
- refactor: remove legacy bash build and environment utility scripts
- ci: add aarch64-unknown-linux-gnu build target and update workflow logic for Ubuntu runners
- fix: rename ximagesrc property from show-cursor to show-pointer
- refactor: update checkout ref logic to handle repository-agnostic builds
- refactor: optimize build workflow by configuring repository checkout and streamlining script execution
- ci: rename workflow and add support for workflow_call with artifact outputs
- feat: add Contributor License Agreement (CLA) to repository root
