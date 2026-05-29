# kadr installer

GUI installer for kadr built with Rust and egui. Downloads `kadr.exe` and `libmpv-2.dll` from GitHub releases at install time.

## Requirements

- Internet connection at install time
- Windows 10 or later

## What it does

- Downloads and installs `kadr.exe` and `libmpv-2.dll` to the chosen install directory
- Creates desktop and Start Menu shortcuts (optional)
- Adds the install directory to the user `PATH` (optional)
- Registers "Open with Kadr" right-click context menu entries for files, folders, and folder backgrounds (optional)
- Sets kadr as the default image / video viewer (optional)
- Writes an uninstall entry to `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\kadr`
- **Update** — re-downloads and replaces `kadr.exe` in-place, keeping all existing settings
- **Uninstall** — removes shortcuts, PATH entry, context menu entries, and schedules directory deletion

## Building

```powershell
cargo build --release -p installer
# The installer is at target/release/installer.exe
```

No need to build kadr separately — binaries are fetched at install time.
