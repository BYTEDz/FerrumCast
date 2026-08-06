<div align="center">

# ferrumcast

**A high-performance, ultra-low-latency screen capture and streaming daemon.**

[![License: PolyForm Noncommercial](https://img.shields.io/badge/License-PolyForm%20Noncommercial-blue.svg?style=for-the-badge)](https://polyformproject.org/licenses/noncommercial/1.0.0/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20|%20Linux-lightgrey?style=for-the-badge)](https://github.com/BYTEDz/ferrumcast)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-CE382A?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![GStreamer](https://img.shields.io/badge/GStreamer-1.20%2B-4A90E2?style=for-the-badge&logo=gstreamer&logoColor=white)](https://gstreamer.freedesktop.org/)

`ferrumcast` is a cross-platform media capture and streaming engine written in Rust and powered by GStreamer. 
It acts as a self-contained daemon that is managed out-of-process via an optimized local IPC signaling socket.

[**Download Releases**](https://github.com/BYTEDz/ferrumcast/releases) • [**GStreamer Pipeline Docs**](https://gstreamer.freedesktop.org/documentation/)

</div>

---

## Core Architecture

The application is architected into two clean execution domains:

1. **Control Plane (Rust):** Manages asynchronous system capability probing, thread-safe dynamic configuration (`ConfigStore`), the local IPC server, CLI parsing, and XDG Desktop Portal session negotiation on Wayland.
2. **Data Plane (GStreamer):** Orchestrates zero-copy GPU memory transitions, color conversions, real-time hardware/software H.264 & H.265/HEVC video encoding, Opus audio packaging, and low-latency RTP/SRTP transmission over UDP.

---

## Codec Support & Hardware Encoders

At startup, `ferrumcast` queries the local GStreamer registry to identify and prioritize hardware-accelerated video encoders. It supports both **H.264 (AVC)** and **H.265 (HEVC)** codecs across all major GPU vendors and software fallbacks:

| Vendor / Platform | H.264 Encoder | H.265 (HEVC) Encoder | Acceleration Type |
| :--- | :--- | :--- | :--- |
| **Linux VA-API (AMD/Intel)** | `vah264enc` (`vah264`) | `vah265enc` (`vah265`) | **Hardware (VA-API / VAMemory)** |
| **NVIDIA GPU** | `nvh264enc` (`nvenc`) | `nvh265enc` (`nvenc_h265`) | **Hardware (NVENC)** |
| **Intel QuickSync** | `qsvh264enc` (`intel_qsv`) | `qsvh265enc` (`intel_qsv_h265`) | **Hardware (QuickSync)** |
| **AMD AMF (Windows)** | `amfh264enc` (`amd_amf`) | `amfh265enc` (`amd_amf_h265`) | **Hardware (AMF)** |
| **Windows Media Foundation** | `mfh264enc` (`windows_mf`) | `mfh265enc` (`windows_mf_h265`) | **Hardware / OS Native** |
| **Software CPU (x265)** | — | `x265enc` (`x265`) | **CPU Software (`libx265`)** |
| **Software CPU (x264)** | `x264enc` (`x264`) | — | **CPU Software (`libx264`)** |

---

## Capture & Audio Backends

### Video Capture Backends
* **Windows (DXGI / Hardware):** Captures desktop frames directly from the GPU via the DXGI Desktop Duplication API (`d3d11screencapturesrc`). Direct3D 11 textures remain inside GPU VRAM for zero-copy performance.
* **Windows (GDI / VM Software Fallback):** Executes an optimized worker grab loop (`BitBlt`) feeding a GStreamer `appsrc`. Automatically activated on Virtual Machines (detected via CPUID) or via the `--gdi` flag.
* **Linux (Wayland):** Negotiates screen capture via the XDG Desktop Portal (`ashpd`), outputting a PipeWire stream node bound to a `pipewiresrc` element.
* **Linux (X11):** Captures root window frames using `ximagesrc` with XDamage extension optimization.

### Audio Backends
* **Windows:** Captures system-wide output audio using WASAPI Loopback (`wasapisrc loopback=true`).
* **Linux:** Captures system audio from PipeWire / PulseAudio (`pulsesrc`).
* **Audio-Only Mode:** Supports streaming standalone Opus audio without spawning video pipelines (`--audio-only`).

---

## Project Structure

- `src/main.rs`: Application entry point, CLI parser, and process lifecycle.
- `src/config.rs`: Configuration management, CLI arguments, and capability probing (`ConfigStore`).
- `src/ipc.rs`: Socket / Named Pipe IPC server for out-of-process control.
- `src/portal.rs`: Wayland session negotiation via XDG Desktop Portal (`ashpd`).
- `src/stream.rs`: High-level GStreamer pipeline management & dynamic controls.
- `src/input.rs`: Windows mouse & touch input execution via `SendInput`.
- `src/pipeline/`: GStreamer pipeline builders.
    - `generic.rs`: Common caps scaling and resolution logic.
    - `encoders.rs`: Encoder definitions for VA-API, NVENC, QSV, AMF, Media Foundation, x264, and x265.
    - `linux.rs`: Linux capture and audio source definitions.
    - `windows.rs`: Windows capture and WASAPI audio source definitions.
- `src/gdi_capture.rs`: Optimized GDI capture fallback for Windows Virtual Machines.

---

## Getting Started

### Building
Compile the production release binary using Cargo:
```bash
cargo build --release
```

### Linux Dependencies
Ensure GStreamer 1.20+ and required plugins are installed:
```bash
# Fedora
sudo dnf install gstreamer1-plugins-base gstreamer1-plugins-good \
    gstreamer1-plugins-bad-free gstreamer1-plugins-bad-freeworld \
    mesa-va-drivers-freeworld x265

# Debian / Ubuntu
sudo apt update
sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
    gstreamer1.0-pipewire
```

---

## Command Line Reference

 At startup, `ferrumcast` can be configured with the following command-line flags:

#### Execution & Pipeline Flags
* `--probe`: Queries local GStreamer registry for available encoders, outputs a serialized JSON `Capabilities` payload, and exits.
* `--encoder <val>`: Force a specific encoder element:
  * H.265 / HEVC: `vah265`, `nvenc_h265`, `intel_qsv_h265`, `amd_amf_h265`, `windows_mf_h265`, `x265`.
  * H.264 / AVC: `vah264`, `nvenc`, `intel_qsv`, `amd_amf`, `windows_mf`, `x264`.
  * `auto` (default): Auto-selects the best hardware GPU encoder available.
* `--host <ip>`: Sets target IPv4/IPv6 destination address for unicast RTP streaming.
* `--audio <true|false>`: Toggles audio capture loopback.
* `--audio-only <true|false>`: Runs in standalone audio streaming mode (bypasses screen capture).
* `--monitor-index <index>`: Selects target display monitor index to stream (`0`, `1`, `2`...).
* `--gdi`: Forces GDI screen capture on Windows (bypasses DXGI Desktop Duplication).

#### Video Format Control
* `--width <px>`: Target output video width.
* `--height <px>`: Target output video height.
* `--fps <fps>`: Target video stream frame rate (`24`, `30`, `60`, `90`, `120`).
* `--colorimetry <tag>`: Explicitly configures GStreamer colorimetry (`2:3:3:3`, `bt709`, `bt601`).

#### Advanced Encoder Tuning
* `--bitrate <kbps>`: Target encoding bitrate in kbps (defaults to `6000` kbps).
* `--audio-bitrate <kbps>`: Target audio encoding bitrate in kbps (defaults to `128` kbps).
* `--srtp-key <hex>`: Enables SRTP encryption using a concatenated master key and salt.
* `--rc-mode <mode>`: Rate control algorithm (`cbr`, `vbr`, `cqp`).
* `--cqp-value <val>`: Constant Quantization Parameter value (`0-51`).
* `--key-int-max <val>`: Maximum keyframe interval / GOP length in frames (defaults to `60`).
* `--bframes <val>`: Set number of B-frames (defaults to `0` for lowest latency).
* `--speed-preset <preset>`: Encoder speed preset (`ultrafast`, `superfast`, `veryfast`, `fast`, `medium`).

---

## IPC Protocol Specification

Control applications manage the engine out-of-process via an IPC socket:
* **Linux:** UNIX Domain Socket bound to `/tmp/ferrumcast.sock` (`0o600` permissions).
* **Windows:** Named Pipe bound to `\\.\pipe\ferrumcast`.

Communication uses newline-delimited (`\n`) JSON payloads.

### Inbound Control Messages
* `STOP_STREAM`: Pauses capture and sets pipeline to `NULL` state.
* `CONFIGURE_STREAM`: Dynamically updates bitrate on active encoder.
* `RESTART_PIPELINE`: Tears down and rebuilds media pipeline in-place (preserves Wayland D-Bus file descriptors).
* `FORCE_KEYFRAME`: Forces immediate generation of an IDR keyframe.
* `SWITCH_DISPLAY`: Switches active monitor display (`"direction": "next"` or `"prev"`).
* `GET_CAPABILITIES`: Requests verified hardware encoder capabilities.

---

## License & Compliance

### PolyForm Noncommercial License 1.0.0
`ferrumcast` is licensed under the **PolyForm Noncommercial License 1.0.0**.
* **Personal & Non-Commercial Use:** Free to use, modify, and run for personal setups and open-source non-commercial projects.
* **Commercial / Enterprise Use:** Commercial use, closed-source bundling, or profiting off this software requires a **BYTEDz Commercial License**. Contact: `support@bytedz.com`.

### Patent & Third-Party Notice
When using hardware encoders (`vah265`, `vah264`, `nvenc`, `qsv`, `amf`, `windows_mf`), video encoding is offloaded directly to the user's GPU hardware and OS drivers, which are licensed by their respective manufacturers (NVIDIA, Intel, AMD, Microsoft).

---

## Support & Maintainers

<div align="center">

<a href="https://github.com/AzharZouhir">
  <img src="https://github.com/AzharZouhir.png" width="100px" style="border-radius: 50%; border: 3px solid #3d76ab;" alt="Azhar Zouhir"/>
</a>

**[Azhar Zouhir](https://github.com/AzharZouhir)**  
_Creator & Lead Developer (BYTEDz)_

[![GitHub](https://img.shields.io/badge/GitHub-181717?style=flat-square&logo=github&logoColor=white)](https://github.com/AzharZouhir) [![Email](https://img.shields.io/badge/Email-D14836?style=flat-square&logo=gmail&logoColor=white)](mailto:support@bytedz.com)

Free Palestine • Made with love in Algeria

</div>