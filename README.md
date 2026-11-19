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
- **Ultra-lightweight:** Written in pure Rust with native Win32 APIs (no heavy runtime, < 8 MB RAM, 0% CPU at idle).
- **Customizable Thresholds:** JSON-based configuration for thermal limits, grace periods, and polling intervals.
- **Automated Windows Installer:** Clean installer created with Inno Setup, supporting auto-start on Windows boot.

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
