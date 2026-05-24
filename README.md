# kadr installer

GUI installer for kadr built with Rust and egui. Embeds `kadr.exe` at compile time and installs it to `%LOCALAPPDATA%\kadr`.

## What it does

- Copies `kadr.exe` to the chosen install directory
- Creates desktop and Start Menu shortcuts (optional)
- Adds the install directory to the user `PATH` (optional)
- Registers "Open with Kadr" right-click context menu entries for files, folders, and folder backgrounds (optional)
- Sets kadr as the default image / video viewer (optional)
- Writes an uninstall entry to `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\kadr`
- **Update** — overwrites the binary in-place, keeping all existing settings
- **Uninstall** — removes shortcuts, PATH entry, context menu entries, and schedules directory deletion

## Building

You must build kadr first so the installer can embed it.

```powershell
# 1. Build kadr
cargo build --release -p kadr

# 2. Build installer (embeds ../target/release/kadr.exe automatically)
cargo build --release -p installer

# The installer is at target/release/installer.exe
```

If `KADR_EXE_SRC` is set, `build.rs` uses that path instead:

```powershell
$env:KADR_EXE_SRC = "C:\path\to\kadr.exe"
cargo build --release -p installer
```

## Running without building kadr first

If `kadr.exe` is not found during the installer build, `build.rs` writes a stub placeholder. The resulting installer will install the stub, not a real binary. Always build kadr first.
