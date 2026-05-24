use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

use anyhow::{Context, Result};

// The kadr.exe binary is embedded at compile time.
// Set KADR_EXE_PATH env var to point at the release build before building installer.
static KADR_EXE: &[u8] = include_bytes!(concat!(env!("KADR_EXE_PATH")));

#[derive(Clone)]
pub struct InstallOptions {
    pub install_dir: PathBuf,
    pub desktop_shortcut: bool,
    pub start_menu_shortcut: bool,
    pub add_to_path: bool,
    pub context_menu: bool,
    pub default_image_viewer: bool,
    pub default_video_viewer: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\Public".to_owned());
        Self {
            install_dir: PathBuf::from(local_app_data).join("kadr"),
            desktop_shortcut: true,
            start_menu_shortcut: true,
            add_to_path: true,
            context_menu: true,
            default_image_viewer: false,
            default_video_viewer: false,
        }
    }
}

pub enum InstallProgress {
    Log(String),
    Step(f32),
    Done,
    Error(String),
}

pub fn detect_existing_install() -> Option<PathBuf> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let dir = PathBuf::from(local_app_data).join("kadr");
    if dir.join("kadr.exe").exists() {
        Some(dir)
    } else {
        None
    }
}

pub fn run_install(opts: &InstallOptions, tx: mpsc::Sender<InstallProgress>) {
    if let Err(e) = do_install(opts, &tx) {
        let _ = tx.send(InstallProgress::Error(format!("{e:#}")));
    } else {
        let _ = tx.send(InstallProgress::Done);
    }
}

pub fn run_update(install_dir: &std::path::Path, tx: mpsc::Sender<InstallProgress>) {
    let _ = tx.send(InstallProgress::Log("Updating kadr…".to_owned()));
    let _ = tx.send(InstallProgress::Step(0.2));

    let exe_path = install_dir.join("kadr.exe");
    match std::fs::write(&exe_path, KADR_EXE) {
        Ok(_) => {
            let _ = tx.send(InstallProgress::Log(format!("Updated {}", exe_path.display())));
            let _ = tx.send(InstallProgress::Step(1.0));
            let _ = tx.send(InstallProgress::Log("Done!".to_owned()));
            let _ = tx.send(InstallProgress::Done);
        }
        Err(e) => {
            let _ = tx.send(InstallProgress::Error(format!("Cannot write {}: {e}", exe_path.display())));
        }
    }
}

fn do_install(opts: &InstallOptions, tx: &mpsc::Sender<InstallProgress>) -> Result<()> {
    let steps: f32 = 7.0;
    let mut step = 0f32;

    let send_log = |msg: &str, tx: &mpsc::Sender<InstallProgress>| {
        let _ = tx.send(InstallProgress::Log(msg.to_owned()));
    };
    let send_step = |s: f32, tx: &mpsc::Sender<InstallProgress>| {
        let _ = tx.send(InstallProgress::Step(s / steps));
    };

    // 1. Create install directory
    send_log("Creating install directory…", tx);
    std::fs::create_dir_all(&opts.install_dir)
        .with_context(|| format!("Cannot create {}", opts.install_dir.display()))?;
    step += 1.0; send_step(step, tx);

    // 2. Write kadr.exe
    let exe_path = opts.install_dir.join("kadr.exe");
    send_log(&format!("Writing {}…", exe_path.display()), tx);
    std::fs::write(&exe_path, KADR_EXE)
        .with_context(|| format!("Cannot write {}", exe_path.display()))?;
    step += 1.0; send_step(step, tx);

    // 3. Desktop shortcut
    if opts.desktop_shortcut {
        send_log("Creating desktop shortcut…", tx);
        let desktop = desktop_dir();
        create_shortcut(&exe_path, &desktop.join("Kadr.lnk"), "Kadr Image Viewer")?;
    }
    step += 1.0; send_step(step, tx);

    // 4. Start menu shortcut
    if opts.start_menu_shortcut {
        send_log("Creating Start Menu shortcut…", tx);
        let sm = start_menu_dir();
        std::fs::create_dir_all(&sm).ok();
        create_shortcut(&exe_path, &sm.join("Kadr.lnk"), "Kadr Image Viewer")?;
    }
    step += 1.0; send_step(step, tx);

    // 5. Add to PATH
    if opts.add_to_path {
        send_log("Adding to user PATH…", tx);
        add_to_user_path(&opts.install_dir)?;
    }
    step += 1.0; send_step(step, tx);

    // 6. Context menu
    if opts.context_menu {
        send_log("Registering context menu…", tx);
        register_context_menu(&exe_path)?;
    }
    step += 1.0; send_step(step, tx);

    // 7. Default viewers + uninstall registry entry
    if opts.default_image_viewer {
        send_log("Setting default image viewer…", tx);
        set_default_image_viewer(&exe_path)?;
    }
    if opts.default_video_viewer {
        send_log("Setting default video viewer…", tx);
        set_default_video_viewer(&exe_path)?;
    }
    register_uninstall_entry(&exe_path, &opts.install_dir)?;
    step += 1.0; send_step(step, tx);

    send_log("Done!", tx);
    Ok(())
}

// ── Shortcuts ────────────────────────────────────────────────────────────────

fn create_shortcut(target: &Path, lnk: &Path, description: &str) -> Result<()> {
    let script = format!(
        r#"$ws = New-Object -ComObject WScript.Shell
$s = $ws.CreateShortcut("{lnk}")
$s.TargetPath = "{target}"
$s.Description = "{description}"
$s.Save()"#,
        lnk = lnk.display(),
        target = target.display(),
        description = description,
    );
    powershell(&script)?;
    Ok(())
}

fn desktop_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Desktop"))
        .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Public\\Desktop"))
}

fn start_menu_dir() -> PathBuf {
    std::env::var("APPDATA")
        .map(|p| PathBuf::from(p).join("Microsoft\\Windows\\Start Menu\\Programs\\Kadr"))
        .unwrap_or_else(|_| PathBuf::from("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Kadr"))
}

// ── PATH ─────────────────────────────────────────────────────────────────────

fn add_to_user_path(dir: &Path) -> Result<()> {
    use winreg::{enums::*, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env_key = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .context("Cannot open HKCU\\Environment")?;

    let current: String = env_key.get_value("Path").unwrap_or_default();
    let dir_str = dir.to_string_lossy();
    if current.split(';').any(|p| p.trim().eq_ignore_ascii_case(&*dir_str)) {
        return Ok(());
    }
    let new_path = if current.is_empty() {
        dir_str.into_owned()
    } else {
        format!("{current};{dir_str}")
    };
    env_key.set_value("Path", &new_path).context("Cannot write PATH")?;

    // Broadcast WM_SETTINGCHANGE so open Explorer / cmd windows pick up the new PATH
    let _ = powershell(
        r#"[System.Environment]::SetEnvironmentVariable('Path', [System.Environment]::GetEnvironmentVariable('Path','User'), 'User')"#,
    );
    Ok(())
}

// ── Context menu ─────────────────────────────────────────────────────────────

fn register_context_menu(exe: &Path) -> Result<()> {
    use winreg::{enums::*, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let icon = format!("\"{}\",0", exe.display());
    let cmd_file = format!("\"{}\" \"%1\"", exe.display());
    let cmd_folder = format!("\"{}\" \"%1\"", exe.display());
    let cmd_bg = format!("\"{}\" \"%V\"", exe.display());

    // Files: right-click on any file
    register_shell_entry(&hkcu, r"Software\Classes\*\shell\Open with Kadr", &cmd_file, &icon)?;

    // Folders: right-click on a folder
    register_shell_entry(&hkcu, r"Software\Classes\Directory\shell\Open with Kadr", &cmd_folder, &icon)?;

    // Folder background: right-click on empty space inside a folder
    register_shell_entry(&hkcu, r"Software\Classes\Directory\Background\shell\Open with Kadr", &cmd_bg, &icon)?;

    Ok(())
}

fn register_shell_entry(hkcu: &winreg::RegKey, base: &str, cmd: &str, icon: &str) -> Result<()> {
    use winreg::enums::*;
    let (cmd_key, _) = hkcu
        .create_subkey(&format!("{base}\\command"))
        .with_context(|| format!("Cannot create {base}\\command"))?;
    cmd_key.set_value("", &cmd.to_owned()).context("Cannot set command")?;

    let menu_key = hkcu
        .open_subkey_with_flags(base, KEY_WRITE)
        .unwrap_or_else(|_| hkcu.create_subkey(base).expect("shell entry key").0);
    menu_key.set_value("Icon", &icon.to_owned()).ok();

    Ok(())
}

// ── Default viewers ───────────────────────────────────────────────────────────

fn set_default_image_viewer(exe: &Path) -> Result<()> {
    register_prog_id(exe, "kadr.image", "Image File")?;
    for ext in &["jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "avif"] {
        set_user_file_assoc(ext, "kadr.image")?;
    }
    Ok(())
}

fn set_default_video_viewer(exe: &Path) -> Result<()> {
    register_prog_id(exe, "kadr.video", "Video File")?;
    for ext in &["mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v"] {
        set_user_file_assoc(ext, "kadr.video")?;
    }
    Ok(())
}

fn register_prog_id(exe: &Path, prog_id: &str, description: &str) -> Result<()> {
    use winreg::{enums::*, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let base = format!("Software\\Classes\\{prog_id}");
    let (key, _) = hkcu.create_subkey(&base).context("prog id root")?;
    key.set_value("", &description.to_owned())?;
    let (cmd, _) = hkcu.create_subkey(&format!("{base}\\shell\\open\\command"))?;
    cmd.set_value("", &format!("\"{}\" \"%1\"", exe.display()))?;
    Ok(())
}

fn set_user_file_assoc(ext: &str, prog_id: &str) -> Result<()> {
    use winreg::{enums::*, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(&format!("Software\\Classes\\.{ext}"))?;
    key.set_value("", &prog_id.to_owned())?;
    Ok(())
}

// ── Uninstall entry ───────────────────────────────────────────────────────────

fn register_uninstall_entry(exe: &Path, install_dir: &Path) -> Result<()> {
    use winreg::{enums::*, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_path =
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\kadr";
    let (key, _) = hkcu.create_subkey(key_path).context("Cannot create uninstall key")?;
    key.set_value("DisplayName", &"Kadr Image Viewer".to_owned())?;
    key.set_value("UninstallString", &format!("\"{}\" --uninstall", exe.display()))?;
    key.set_value("InstallLocation", &install_dir.to_string_lossy().to_string())?;
    key.set_value("DisplayIcon", &format!("\"{}\",0", exe.display()))?;
    key.set_value("Publisher", &"".to_owned())?;
    key.set_value("NoModify", &1u32)?;
    key.set_value("NoRepair", &1u32)?;
    Ok(())
}

// ── PowerShell helper ─────────────────────────────────────────────────────────

fn powershell(script: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("Cannot run PowerShell")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("PowerShell error: {stderr}");
    }
    Ok(())
}
