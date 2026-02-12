# B24 Tool

A desktop configuration and calibration tool for **Mantracourt B24 BLE telemetry transmitters**.

Built with Rust, egui, and btleplug.

## Features

- **Device Discovery** — Scan for B24 transmitters via BLE (BLED112 dongle or Windows Bluetooth)
- **Configuration** — Read/write all device registers: data rate, resolution, units, PINs, local name, and more
- **Calibration** — Two-point auto calibration, multi-point table calibration, and direct register access
- **Live Data** — Real-time value display with status flags, time-series plotting, and adjustable decimal places
- **Data Logging** — Record timestamped data with CSV export
- **Mobile Export** — Generate JSON configs and QR codes for the B24 mobile app
- **View Mode** — Fullscreen data display for both connected and advertising (PIN-decoded) devices
- **Import/Export** — Save and restore device configurations (JSON and text formats)
- **Light/Dark Theme** — Toggle between themes (preference saved across sessions)
- **Auto-Update Check** — Notifies when a new release is available on GitHub

## Requirements

### Hardware

- **Mantracourt B24 transmitter** — the device you want to configure
- **BLED112 USB BLE dongle** (recommended) — Silicon Labs Bluegiga BLED112 for fast, reliable scanning
- Alternatively, any Windows-compatible Bluetooth LE adapter will work via Windows Bluetooth

### Software

- **Windows 10** or later (64-bit)

## Installation

### Option 1: Installer (recommended)

1. Go to [Releases](https://github.com/ClarkeFL/b24studio/releases/latest)
2. Download `B24-Tool-{version}-Setup.exe`
3. Run the installer — it will install to `Program Files\B24 Tool` and create Start Menu shortcuts

### Option 2: Standalone executable

1. Go to [Releases](https://github.com/ClarkeFL/b24studio/releases/latest)
2. Download `b24-tool-{version}-windows-x64.zip`
3. Extract and run `b24-tool.exe` — no installation required

## Usage

1. Plug in your BLED112 dongle (or enable Windows Bluetooth)
2. Launch B24 Tool
3. Click **Start Scan** on the Connect tab to discover nearby B24 devices
4. Select a device and enter the Config PIN (default: `0`) to connect
5. Use the tabs to configure, calibrate, and monitor your transmitter

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) stable toolchain (`x86_64-pc-windows-msvc`)
- [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/) with the "Desktop development with C++" workload
- Windows 10 SDK

### Build

```bash
# Ensure MSVC linker is on PATH (Git's link.exe can shadow it)
set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%PATH%

# Build release binary
cargo build --release
```

The executable will be at `target\release\b24-tool.exe`.

## Project Structure

```
src/
  app.rs            # Application logic and BLE event processing
  main.rs           # Entry point and window setup
  state.rs          # Centralised application state
  ble/              # BLE communication (btleplug + BlueGiga BLED112)
  protocol/         # B24 protocol codec, UUIDs, and types
  ui/               # egui UI components (one file per tab)
assets/             # Application icons
installer/          # Inno Setup installer script
.github/workflows/  # CI/CD release automation
```

## License

MIT
