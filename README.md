# mrec

Portable Windows audio recorder. Records system audio + microphone to MP3.

## Features

- Records system audio (WASAPI loopback) and/or microphone
- MP3 encoding (128/192/256/320 kbps)
- System tray icon with left-click toggle (start/stop recording)
- Settings window: output folder, quality, audio source, mic selection, volume controls, filename template
- Portable: single .exe, no installation, settings saved to `mrec.json` next to .exe
- Recordings saved to configurable folder (default: `recordings/` next to .exe)

## Usage

1. Download `mrec.exe` from [Releases](https://github.com/pilotv/mrec/actions)
2. Run — gray circle appears in system tray
3. **Left-click** tray icon — toggle recording on/off
4. **Right-click** tray icon — menu:
   - **Start Recording** / **Stop Recording**
   - **Settings...** — configure output folder, quality, source, mic, volumes
   - **Exit**
5. Recording indicator: gray circle = idle, red circle = recording

## Settings

| Setting | Options | Default |
|---------|---------|---------|
| Output folder | Any folder (Browse) | `recordings/` next to .exe |
| MP3 Quality | 128 / 192 / 256 / 320 kbps | 192 kbps |
| Audio source | System + Mic / System only / Mic only | System + Mic |
| Microphone | (Default) or specific device | (Default) |
| System volume | 50% / 75% / 100% / 125% / 150% | 100% |
| Mic volume | 50% / 75% / 100% / 150% / 200% / 300% | 150% |
| Filename | Template with `{date}` and `{time}` | `mrec_{date}_{time}` |

Settings are saved to `mrec.json` next to the .exe.

## Filename Examples

| Template | Result |
|----------|--------|
| `mrec_{date}_{time}` | `mrec_2026-04-03_15-30-00.mp3` |
| `meeting_{date}` | `meeting_2026-04-03.mp3` |
| `rec_{time}` | `rec_15-30-00.mp3` |

## Build

Requires: Rust toolchain, vcpkg with mp3lame.

```bash
vcpkg install mp3lame:x64-windows-static
set VCPKG_ROOT=C:\path\to\vcpkg
set VCPKGRS_DYNAMIC=0
cargo build --release
```

Or push to GitHub — Actions builds automatically.

## Architecture

```
mrec.exe
├── main.rs          — tray icon, Win32 message loop, config management
├── capture.rs       — WASAPI loopback + microphone capture (cpal)
├── mixer.rs         — audio stream mixing with volume control + resampling
├── encoder.rs       — MP3 encoding (mp3lame-encoder)
├── recorder.rs      — orchestrator: capture → mix → encode pipeline
├── config.rs        — settings struct, JSON load/save
└── settings_ui.rs   — native Win32 settings dialog (native-windows-gui)
```

## Requirements

- Windows 10/11
- Audio output device (for system audio capture)
- Microphone (optional, for mic recording)
