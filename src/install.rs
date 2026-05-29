use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc,
};

use anyhow::{Context, Result};

const BUNDLED_KADR_VERSION: &str = env!("KADR_VERSION");

// ── Download config ───────────────────────────────────────────────────────────

const CONFIG_STR: &str = include_str!("../downloads.toml");

#[derive(serde::Deserialize, Clone)]
pub struct DownloadEntry {
    pub url: String,
    pub sha256: String,
}

#[derive(serde::Deserialize)]
struct DownloadConfig {
    files: Vec<DownloadEntry>,
}

pub fn load_config() -> Vec<DownloadEntry> {
    toml::from_str::<DownloadConfig>(CONFIG_STR)
        .expect("Invalid downloads.toml")
        .files
}

pub fn filename_from_url(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or("unknown")
}

fn sha256_of_file(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path).ok()?;
    Some(Sha256::digest(&data).iter().map(|b| format!("{b:02x}")).collect())
}

fn needs_download(entry: &DownloadEntry, install_dir: &Path) -> bool {
    let path = install_dir.join(filename_from_url(&entry.url));
    if !path.exists() { return true; }
    sha256_of_file(&path).map_or(true, |h| h != entry.sha256)
}

pub fn get_pending_updates(install_dir: &Path) -> Vec<DownloadEntry> {
    load_config().into_iter()
        .filter(|e| needs_download(e, install_dir))
        .collect()
}

pub fn fetch_remote_sizes() -> HashMap<String, u64> {
    load_config().iter()
        .filter_map(|e| {
            let name = filename_from_url(&e.url).to_owned();
            let size = head_content_length(&e.url)?;
            Some((name, size))
        })
        .collect()
}
const KADR_REG_KEY: &str = r"Software\Kadr";

pub struct ExistingInstall {
    pub dir: PathBuf,
    pub version: Option<String>,
}

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

pub fn detect_existing_install() -> Option<ExistingInstall> {
    use winreg::{enums::*, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Registry is the primary source — handles custom install dirs.
    if let Ok(key) = hkcu.open_subkey(KADR_REG_KEY) {
        if let Ok(path_str) = key.get_value::<String, _>("InstallPath") {
            let dir = PathBuf::from(&path_str);
            if dir.join("kadr.exe").exists() {
                let version = key.get_value::<String, _>("InstalledVersion").ok();
                return Some(ExistingInstall { dir, version });
            }
        }
    }

    // Fall back to default location for pre-registry installs.
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let dir = PathBuf::from(local_app_data).join("kadr");
    if dir.join("kadr.exe").exists() {
        Some(ExistingInstall { dir, version: None })
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
    let pending = get_pending_updates(install_dir);
    if pending.is_empty() {
        let _ = tx.send(InstallProgress::Log("Already up to date.".to_owned()));
        let _ = tx.send(InstallProgress::Done);
        return;
    }
    let n = pending.len() as f32;
    for (i, entry) in pending.iter().enumerate() {
        let start = i as f32 / n * 0.95;
        let end = (i as f32 + 1.0) / n * 0.95;
        let filename = filename_from_url(&entry.url);
        if let Err(e) = download_file(&entry.url, &install_dir.join(filename), filename, start, end, &tx) {
            let _ = tx.send(InstallProgress::Error(format!("{e:#}")));
            return;
        }
    }
    let _ = write_install_registry(install_dir);

    // Recreate shortcuts so the updated exe icon is picked up
    let exe_path = install_dir.join("kadr.exe");
    let desktop_lnk = desktop_dir().join("Kadr.lnk");
    if desktop_lnk.exists() {
        let _ = create_shortcut(&exe_path, &desktop_lnk, "kadr");
    }
    let sm_lnk = start_menu_dir().join("Kadr.lnk");
    if sm_lnk.exists() {
        let _ = create_shortcut(&exe_path, &sm_lnk, "kadr");
    }
    refresh_icon_cache();

    let _ = tx.send(InstallProgress::Step(1.0));
    let _ = tx.send(InstallProgress::Log("Done!".to_owned()));
    let _ = tx.send(InstallProgress::Done);
}

fn do_install(opts: &InstallOptions, tx: &mpsc::Sender<InstallProgress>) -> Result<()> {
    let steps: f32 = 8.0;
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

    // 2. Download all configured files
    let entries = load_config();
    let n_files = entries.len() as f32;
    send_log("Downloading files…", tx);
    for (i, entry) in entries.iter().enumerate() {
        let start = (step + i as f32 / n_files) / steps;
        let end = (step + (i as f32 + 1.0) / n_files) / steps;
        let filename = filename_from_url(&entry.url);
        download_file(&entry.url, &opts.install_dir.join(filename), filename, start, end, tx)?;
    }
    step += n_files; send_step(step, tx);

    let exe_path = opts.install_dir.join("kadr.exe");

    // 4. Desktop shortcut
    if opts.desktop_shortcut {
        send_log("Creating desktop shortcut…", tx);
        let desktop = desktop_dir();
        create_shortcut(&exe_path, &desktop.join("Kadr.lnk"), "kadr")?;
    }
    step += 1.0; send_step(step, tx);

    // 5. Start menu shortcut
    if opts.start_menu_shortcut {
        send_log("Creating Start Menu shortcut…", tx);
        let sm = start_menu_dir();
        std::fs::create_dir_all(&sm).ok();
        create_shortcut(&exe_path, &sm.join("Kadr.lnk"), "kadr")?;
    }
    step += 1.0; send_step(step, tx);

    // 6. Add to PATH
    if opts.add_to_path {
        send_log("Adding to user PATH…", tx);
        add_to_user_path(&opts.install_dir)?;
    }
    step += 1.0; send_step(step, tx);

    // 7. Context menu
    if opts.context_menu {
        send_log("Registering context menu…", tx);
        register_context_menu(&exe_path)?;
    }
    step += 1.0; send_step(step, tx);

    // 8. Default viewers + uninstall registry entry
    if opts.default_image_viewer {
        send_log("Setting default image viewer…", tx);
        set_default_image_viewer(&exe_path)?;
    }
    if opts.default_video_viewer {
        send_log("Setting default video viewer…", tx);
        set_default_video_viewer(&exe_path)?;
    }
    register_uninstall_entry(&exe_path, &opts.install_dir)?;
    write_install_registry(&opts.install_dir)?;
    refresh_icon_cache();
    step += 1.0; send_step(step, tx);

    send_log("Done!", tx);
    Ok(())
}

// ── Download ──────────────────────────────────────────────────────────────────

fn head_content_length(url: &str) -> Option<u64> {
    ureq::head(url).call().ok()
        .and_then(|r| {
            r.headers().get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
        })
}

fn download_file(
    url: &str,
    dest: &Path,
    label: &str,
    progress_start: f32,
    progress_end: f32,
    tx: &mpsc::Sender<InstallProgress>,
) -> Result<()> {
    let _ = tx.send(InstallProgress::Log(format!("Downloading {label}…")));
    let resp = ureq::get(url).call()
        .with_context(|| format!("Failed to download {label}"))?;
    let total: Option<u64> = resp.headers().get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());
    let mut reader = resp.into_body().into_reader();
    let mut data: Vec<u8> = if let Some(t) = total { Vec::with_capacity(t as usize) } else { Vec::new() };
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf).context("Read error during download")?;
        if n == 0 { break; }
        data.extend_from_slice(&buf[..n]);
        if let Some(t) = total {
            let frac = progress_start + (data.len() as f32 / t as f32) * (progress_end - progress_start);
            let _ = tx.send(InstallProgress::Step(frac));
        }
    }
    std::fs::write(dest, &data)
        .with_context(|| format!("Cannot write {}", dest.display()))?;
    let _ = tx.send(InstallProgress::Log(format!("Installed {label}")));
    Ok(())
}

// ── Shortcuts ────────────────────────────────────────────────────────────────

fn create_shortcut(target: &Path, lnk: &Path, description: &str) -> Result<()> {
    let script = format!(
        r#"$ws = New-Object -ComObject WScript.Shell
$s = $ws.CreateShortcut("{lnk}")
$s.TargetPath = "{target}"
$s.IconLocation = "{target},0"
$s.Description = "{description}"
$s.Save()"#,
        lnk = lnk.display(),
        target = target.display(),
        description = description,
    );
    powershell(&script)?;
    Ok(())
}

fn refresh_icon_cache() {
    let _ = powershell(r#"& "$env:SystemRoot\System32\ie4uinit.exe" -show"#);
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
    key.set_value("DisplayName", &"kadr".to_owned())?;
    key.set_value("DisplayVersion", &BUNDLED_KADR_VERSION.to_owned())?;
    key.set_value("UninstallString", &format!("\"{}\" --uninstall", exe.display()))?;
    key.set_value("InstallLocation", &install_dir.to_string_lossy().to_string())?;
    key.set_value("DisplayIcon", &format!("\"{}\",0", exe.display()))?;
    key.set_value("Publisher", &"".to_owned())?;
    key.set_value("NoModify", &1u32)?;
    key.set_value("NoRepair", &1u32)?;
    Ok(())
}

// ── Registry ─────────────────────────────────────────────────────────────────

fn write_install_registry(install_dir: &Path) -> Result<()> {
    use winreg::{enums::*, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(KADR_REG_KEY).context("Cannot create Kadr registry key")?;
    key.set_value("InstallPath", &install_dir.to_string_lossy().to_string())?;
    key.set_value("InstalledVersion", &BUNDLED_KADR_VERSION.to_owned())?;
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
