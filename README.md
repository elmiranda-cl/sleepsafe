<p align="center">
  <img src="assets/sleepsafe_256.png" width="160" alt="SleepSafe Logo" />
</p>

# SleepSafe

> Intelligent thermal and closed-lid sleep protection for Windows laptops.

## What is SleepSafe?

On Windows laptops, when you configure **"When I close the lid: Do nothing"** while connected to AC power, Windows fails to re-evaluate the lid state if the charger is later disconnected or if a power outage occurs.

If you unplug your laptop while the lid is already closed and slip it into a backpack, the machine stays awake, the vents get blocked, and heat rapidly builds up. This leads to battery drain, severe thermal throttling, or hardware damage (commonly known as the **"Backpack Oven Syndrome"**).

**SleepSafe** solves this by running as a lightweight background guard:
1. **Unplug Detection:** When the AC charger is disconnected with the lid closed, SleepSafe initiates thermal monitoring.
2. **Backpack vs Desk Distinguisher:** 
   - On a desk during a power outage, ambient airflow keeps temperatures cool and stable. SleepSafe provides a 10-minute grace window in case power returns.
   - Inside a backpack, fabric blocks airflow and temperatures climb steadily. SleepSafe detects this upward thermal slope (or an absolute ceiling of 58°C) and **immediately suspends the machine** to protect your hardware.
3. **External Monitor Awareness:** If an external display (HDMI/DisplayPort) is attached, SleepSafe recognizes the laptop is docked on a desk and bypasses thermal alarms.

---

## Features
- **Native System Tray Integration:** Sits quietly beside the clock with custom shield icon, live tooltips, sensor diagnostics, and instant context controls (view live logs, open config folder, pause/resume, toggle auto-start, and exit).
- **Auto-Start with Windows:** Easily toggled directly from the tray icon or config file, automatically syncing with `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- **Ultra-lightweight:** Written in pure Rust with native Win32 APIs (no heavy runtime, < 8 MB RAM, 0% CPU at idle).
- **Customizable Thresholds:** JSON-based configuration (`%APPDATA%\SleepSafe\config.json`) for thermal limits, grace periods, and polling intervals.
- **Embedded Windows Metadata:** Complete multi-resolution icon resources (16x16 to 256x256) and official Windows VersionInfo properties.

---

## Installation

### Option 1: Standalone Installer (Recommended)
Download `sleepsafe-setup.exe` from the latest [GitHub Releases](https://github.com/elmiranda-cl/sleepsafe/releases/latest). Run it to install SleepSafe in your user directory and enable auto-start on Windows boot.

### Option 2: Portable Executable
Download `sleepsafe.exe` directly, place it anywhere you like, and run it. You can toggle "Start automatically with Windows" directly from the system tray menu.

### Option 3: Winget (Windows Package Manager)
```cmd
winget install elmiranda-cl.SleepSafe
```

---

## Development & Building

### Prerequisites
- [Rust](https://www.rust-lang.org/) (stable toolchain)
- Windows 10 / 11

### Running Tests
```bash
cargo test
```

### Compiling Release Binary
```bash
cargo build --release
```

The compiled executable will be located at `target/release/sleepsafe.exe`.

---

## License
MIT License - Developed by elmiranda-cl.
