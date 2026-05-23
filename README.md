<div align="center">

# ferrumcast

**A high-performance, ultra-low-latency screen capture and streaming daemon.**

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg?style=for-the-badge)](https://www.gnu.org/licenses/agpl-3.0)
[![Platform](https://img.shields.io/badge/Platform-Windows%20|%20Linux-lightgrey?style=for-the-badge)](https://github.com/BYTEDz/ferrumcast)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-CE382A?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![GStreamer](https://img.shields.io/badge/GStreamer-1.20%2B-4A90E2?style=for-the-badge&logo=gstreamer&logoColor=white)](https://gstreamer.freedesktop.org/)

`ferrumcast` is a cross-platform streaming daemon written in Rust and powered by GStreamer. 
It acts as a self-contained capture engine that is managed out-of-process via an optimized IPC signaling socket.

[**Download Releases**](https://github.com/BYTEDz/ferrumcast/releases) • [**API Architecture**](https://github.com/BYTEDz/ferrumcast/wiki) • [**GStreamer Pipeline Docs**](https://gstreamer.freedesktop.org/documentation/)

</div>

> [!WARNING]
> **Early Build Stage Notice:** `ferrumcast` is currently in an active, early development stage (alpha/pre-release). Due to the highly hardware-dependent nature of real-time video encoders, low-level graphics capture APIs, and platform combinations, this engine may not function properly on all system configurations. Bug reports, diagnostics, and contributions are highly welcome.

## Core Architecture

The application is architected into two clean execution domains:

1.  **Control Plane (Rust):** Manages asynchronous system capability probing, thread-safe dynamic configuration (`ConfigStore`), the local IPC server, and XDG Desktop Portal session negotiation on Wayland.
2.  **Data Plane (GStreamer):** Manages the physical media pipelines, orchestrating zero-copy GPU memory transitions, color conversions, real-time audio/video packaging (RTP or WebRTC), and adaptive network transmission.

## Capabilities

### Capture Backends
*   **Windows (DXGI / Hardware):** Captures desktop frames directly from the GPU via the DXGI Desktop Duplication API (`d3d11screencapturesrc`). Direct3D 11 textures are kept inside GPU memory to maximize performance and minimize CPU overhead.
*   **Windows (GDI / Software Fallback):** Executes an optimized, background-worker grab loop (`BitBlt`) feeding a GStreamer `appsrc`. This loop targets ~30 FPS, overlays the system cursor with accurate hotspot alignments, and implements strict resource disposal to prevent GDI handle leaks.
*   **Linux (Wayland):** Negotiates screen capture via the XDG Desktop Portal and `ashpd`. This outputs a PipeWire stream node and file descriptor which are dynamically bound to a `pipewiresrc` element.
*   **Linux (X11):** Captures frames from the root window using `ximagesrc` with the XDamage extension enabled to optimize redraw cycles.

### Audio Backends
*   **Windows:** Captures system-wide output audio using WASAPI Loopback (`wasapisrc loopback=true`).
*   **Linux:** Captures system audio from the local sound server using PulseAudio (`pulsesrc`).

### Codec Support and Hardware Resolution
At startup, `ferrumcast` queries the GStreamer registry to identify and prioritize available hardware H.264 encoders. If no hardware acceleration block is detected, it falls back to a software encoder:
*   **NVIDIA NVENC:** `nvh264enc`
*   **Intel QuickSync (QSV):** `qsvh264enc`
*   **AMD AMF:** `amfh264enc`
*   **Linux VA-API:** `vah264enc`
*   **Windows Media Foundation:** `mfh264enc` (standard Windows software/hardware fallback)
*   **VideoLAN x264:** `x264enc` (universal software encoder fallback)

### Network Transport Protocols
*   **WebRTC:** Supports complete peer negotiation (SDP and ICE candidate exchange) in-pipeline via `webrtcbin` using a public STUN server fallback.
*   **RTP over UDP:** Provides raw, direct, low-overhead H.264 and Opus RTP streaming straight to a target receiver port.

## Deployment & Dependencies

### Windows (Portable Deployment)
`ferrumcast` can run entirely self-contained without requiring a system-wide GStreamer installation. 
At startup, the executable detects its parent folder and searches for a local `lib/gstreamer-1.0` plugins folder and `libexec/gstreamer-1.0/gst-plugin-scanner.exe`. If found, the engine dynamically overrides its environment variables (`GST_PLUGIN_PATH`, `GST_PLUGIN_SCANNER`, and `PATH`) to execute within this isolated runtime, facilitating easy packaging within a single installer payload.

### Linux
The daemon dynamically links to the host system’s GStreamer libraries. It requires GStreamer 1.20+ alongside GStreamer's Base, Good, Bad, and Ugly plugin sets installed via the system package manager.

## Getting Started

### Prerequisites

#### Windows
Ensure you have the GStreamer MSVC binaries installed, or distribute the required portable runtime layout in your application directory alongside the binary.

#### Linux (Debian/Ubuntu)
```bash
sudo apt update
sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
    gstreamer1.0-pipewire
```

### Building
Compile the production release binary using Cargo:
```bash
cargo build --release
```

### Command Line Reference

At startup, the daemon can be configured with the following command-line arguments:

#### Basic Execution Flags
*   `--probe`: Queries the local GStreamer registry for available encoders, prints the results as a serialized JSON `Capabilities` payload, and exits.
*   `--encoder <val>`: Force a specific encoder element (`vah264`, `nvenc`, `qsv`, `amf`, `mf`, `x264`, or `auto`).
*   `--output <mode>`: Sets the pipeline's network output protocol (`rtp` or `webrtc`).
*   `--host <ip>`: Sets the target IPv4/IPv6 destination address for raw RTP streaming.
*   `--audio <true|false>`: Enable or disable audio capture loopback.
*   `--gdi`: Forces GDI screen capture fallback on Windows, bypassing DXGI.

#### Video Format Control
*   `--width <px>`: Scales the captured screen content to a target width.
*   `--height <px>`: Scales the captured screen content to a target height.
*   `--fps <fps>`: Throttles or interpolates target video stream frame rate.
*   `--show-cursor <true|false>`: Toggles hardware/software cursor overlay visibility.
*   `--colorimetry <tag>`: Explicitly configures GStreamer colorimetry tags (`bt709`, `bt601`, `bt2020`).

#### Advanced Encoder Tuning
*   `--bitrate <kbps>`: Target encoding bitrate (defaults to `6000` kbps).
*   `--rc-mode <mode>`: Chooses the encoder's rate control algorithm (`cbr`, `vbr`, `cqp`).
*   `--cqp-value <val>`: Constant Quantization Parameter value (active only when `rc_mode` is `cqp`).
*   `--key-int-max <val>`: Maximum keyframe interval (GOP length) in frames (defaults to `60`).
*   `--bframes <val>`: Set the number of consecutive B-frames (defaults to `0` for lowest latency).
*   `--ref-frames <val>`: Set the number of reference frames (defaults to `1` for lowest latency).
*   `--speed-preset <preset>`: Speed/Quality tuning preset for `x264` (e.g., `ultrafast`).
*   `--tune <tune>`: Tuning configuration for `x264` (e.g., `zerolatency`).
*   `--nvenc-preset <preset>`: NVENC-specific quality preset (`p1` ... `p7`).
*   `--nvenc-tune <tune>`: NVENC-specific tuning parameters (`ultra-low-latency`, `low-latency`, `high-quality`).
*   `--vaapi-target-usage <1-7>`: VAAPI performance preset (`1` = fastest, `7` = highest quality).
*   `--qsv-target-usage <1-7>`: QSV performance preset (`1` = fastest, `7` = balanced).

#### Network and Queue Controls
*   `--rtp-mtu <bytes>`: Bounds maximum IP packet MTU size (defaults to `1200`).
*   `--queue-max-time-ns <ns>`: Strict latency limit cap inside queue elements. Set to `0` to disable.
*   `--queue-max-buffers <val>`: Max queue buffer count limit to prevent memory bloating during network jitter.
*   `--aggregate-mode <mode>`: Configures packet aggregation (`zero-latency`, `none`, `next-keyframe`).
*   `--udp-buffer-size <bytes>`: Sets the OS-level UDP transmission buffer size.

## Wayland Session Restoration

On Wayland, screen capture requires explicit user authorization via D-Bus. To eliminate redundant, intrusive permission dialogs during subsequent runs:
1.  On first run, a system permission dialog is displayed.
2.  Once accepted, `ferrumcast` receives a unique `restore_token` from the portal.
3.  This token is automatically cached locally in `/tmp/ferrumcast.token`.
4.  Subsequent executions read this token on startup, allowing silent, immediate session restoration without any user-facing prompts.

## IPC Protocol Specification

Control applications manage the engine out-of-process via an IPC socket:
*   **Linux:** UNIX Domain Socket bound to `/tmp/ferrumcast.sock` (enforcing highly restrictive `0o600` permissions to prevent local privilege escalation).
*   **Windows:** Named Pipe bound to `\\.\pipe\ferrumcast` (enforcing a strict `first-pipe-instance` check to prevent named pipe hijacking or spoofing).

Communication consists of single-line, newline-delimited (`\n`) JSON payloads. Payload processing is strictly bounded to a maximum read length of 1MB per line to mitigate Denial of Service (DoS) and Out-Of-Memory (OOM) vulnerabilities.

### Inbound Messages (Control Plane Input)
*   `SET_REMOTE_SDP`:
    ```json
    { "type": "SET_REMOTE_SDP", "sdp": "...SDP TEXT...", "sdp_type": "answer" }
    ```
*   `ADD_ICE_CANDIDATE`:
    ```json
    { "type": "ADD_ICE_CANDIDATE", "candidate": "...", "sdpMid": null, "sdpMLineIndex": 0 }
    ```
*   `REQUEST_OFFER`:
    ```json
    { "type": "REQUEST_OFFER" }
    ```
*   `STOP_STREAM`: Set the GStreamer pipeline to the `NULL` state, pausing capture but leaving the IPC daemon active.
    ```json
    { "type": "STOP_STREAM" }
    ```
*   `CONFIGURE_STREAM`: Updates configuration parameters (such as live bitrate throttling) on an active pipeline.
    ```json
    { "type": "CONFIGURE_STREAM", "bitrate": 8000 }
    ```
*   `RESTART_PIPELINE`: Dynamically tear down and rebuild the media pipeline in-place (such as shifting output modes or changing encoders) without restarting the process. This maintains the Wayland portal D-Bus file descriptor.
    ```json
    { "type": "RESTART_PIPELINE", ... "StreamConfig" ... }
    ```
*   `FORCE_KEYFRAME`: Forces the active encoder element to immediately emit an IDR keyframe (vital for recovering newly joined clients on lossy connections).
    ```json
    { "type": "FORCE_KEYFRAME" }
    ```
*   `GET_CAPABILITIES`: Requests the capabilities payload.
    ```json
    { "type": "GET_CAPABILITIES" }
    ```

### Outbound Messages (Control Plane Output)
*   `LOCAL_SDP_GENERATED`: Dispatches local WebRTC SDP offers for signaling.
    ```json
    { "type": "LOCAL_SDP_GENERATED", "sdp": "...SDP...", "sdp_type": "offer" }
    ```
*   `LOCAL_ICE_CANDIDATE`: Dispatches gathered local ICE candidates.
    ```json
    { "type": "LOCAL_ICE_CANDIDATE", "candidate": "...", "sdpMid": null, "sdpMLineIndex": 0 }
    ```
*   `CAPABILITIES_RESPONSE`: Returns verified system video encoder cap elements.
    ```json
    { "type": "CAPABILITIES_RESPONSE", "encoders": ["nvenc", "x264"] }
    ```
*   `CONFIG_ACK`: Confirms configuration updates and reports the active encoder in use.
    ```json
    { "type": "CONFIG_ACK", "active_encoder": "nvh264enc" }
    ```
*   `PORTAL_TOKEN_GENERATED`: Reports a generated Wayland screencast session restore token.
    ```json
    { "type": "PORTAL_TOKEN_GENERATED", "token": "..." }
    ```
*   `WAITING_FOR_PORTAL_APPROVAL`: Sent when waiting for user interaction on the Wayland display authorization portal.
    ```json
    { "type": "WAITING_FOR_PORTAL_APPROVAL" }
    ```
*   `STREAM_ERROR`: Dispatches pipeline-level failures or initialization errors.
    ```json
    { "type": "STREAM_ERROR", "message": "Failed to resolve video encoder" }
    ```

## License & Compliance

This project is licensed under the **GNU Affero General Public License v3.0 (AGPLv3)**.

### AGPLv3 Copyright Compliance
Because `ferrumcast` is licensed under the AGPLv3, you are legally permitted to distribute the GPL-licensed **VideoLAN `x264` library** (`x264-164.dll`) and GStreamer's `libgstx264.dll` plugin within your Windows portable package without violating copyleft terms.

### H.264 Patent Compliance
If your application uses hardware-accelerated encoders (`nvh264enc`, `qsvh264enc`, `amfh264enc`) or routes software-encoding fallbacks through Microsoft's built-in **Windows Media Foundation** (`mfh264enc`), the required patent royalties are already covered directly by the respective hardware and operating system vendors (NVIDIA, Intel, AMD, and Microsoft), leaving your deployment free of patent liabilities. Software-based encoding fallback via `x264` falls under the Via Licensing Alliance (Via LA) royalty-free distribution threshold of up to 100,000 units per year.

## Support & Maintainers

<div align="center">

<a href="https://github.com/AzharZouhir">
  <img src="https://github.com/AzharZouhir.png" width="100px" style="border-radius: 50%; border: 3px solid #3d76ab;" alt="Azhar Zouhir"/>
</a>

**[Azhar Zouhir](https://github.com/AzharZouhir)**
_Creator & Lead Developer_
Building the next generation of PC remote management.

[![GitHub](https://img.shields.io/badge/GitHub-181717?style=flat-square&logo=github&logoColor=white)](https://github.com/AzharZouhir) [![Email](https://img.shields.io/badge/Email-D14836?style=flat-square&logo=gmail&logoColor=white)](mailto:support@bytedz.com)

Free Palestine • Made with love in Algeria

</div>