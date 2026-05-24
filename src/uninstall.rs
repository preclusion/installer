use std::{path::Path, sync::mpsc};

use anyhow::{Context, Result};

use crate::install::InstallProgress;

pub fn run_uninstall(install_dir: &Path, tx: mpsc::Sender<InstallProgress>) {
    if let Err(e) = do_uninstall(install_dir, &tx) {
        let _ = tx.send(InstallProgress::Error(format!("{e:#}")));
    } else {
        let _ = tx.send(InstallProgress::Done);
    }
}

fn do_uninstall(install_dir: &Path, tx: &mpsc::Sender<InstallProgress>) -> Result<()> {
    let steps = 5f32;
    let mut step = 0f32;

    macro_rules! log {
        ($msg:expr) => {
            let _ = tx.send(InstallProgress::Log($msg.to_owned()));
        };
    }
    macro_rules! advance {
        () => {
            step += 1.0;
            let _ = tx.send(InstallProgress::Step(step / steps));
        };
    }

    // 1. Remove context menu
    log!("Removing context menu…");
    remove_context_menu();
    advance!();

    // 2. Remove from PATH
    log!("Removing from PATH…");
    remove_from_path(install_dir);
    advance!();

    // 3. Remove shortcuts
    log!("Removing shortcuts…");
    remove_shortcuts();
    advance!();

    // 4. Remove uninstall registry entry + file assocs
    log!("Cleaning registry…");
    remove_uninstall_entry();
    remove_file_assocs();
    advance!();

    // 5. Delete install directory (schedule if exe is running)
    log!(&format!("Deleting {}…", install_dir.display()));
    delete_install_dir(install_dir)?;
    advance!();

    log!("Uninstall complete.");
    Ok(())
}

fn remove_context_menu() {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(r"Software\Classes\*\shell\Open with Kadr");
    let _ = hkcu.delete_subkey_all(r"Software\Classes\Directory\shell\Open with Kadr");
    let _ = hkcu.delete_subkey_all(r"Software\Classes\Directory\Background\shell\Open with Kadr");
}

fn remove_from_path(install_dir: &Path) {
    use winreg::{enums::*, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE) {
        if let Ok(current) = key.get_value::<String, _>("Path") {
            let dir_str = install_dir.to_string_lossy();
            let new: Vec<&str> = current
                .split(';')
                .filter(|p| !p.trim().eq_ignore_ascii_case(&*dir_str))
                .collect();
            let _ = key.set_value("Path", &new.join(";"));
        }
    }
}

fn remove_shortcuts() {
    let targets = [
        std::env::var("USERPROFILE")
            .map(|p| std::path::PathBuf::from(p).join("Desktop\\Kadr.lnk"))
            .unwrap_or_default(),
        std::env::var("APPDATA")
            .map(|p| {
                std::path::PathBuf::from(p)
                    .join("Microsoft\\Windows\\Start Menu\\Programs\\Kadr")
            })
            .unwrap_or_default(),
    ];
    for p in &targets {
        if p.is_file() {
            let _ = std::fs::remove_file(p);
        } else if p.is_dir() {
            let _ = std::fs::remove_dir_all(p);
        }
    }
}

fn remove_uninstall_entry() {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\kadr",
    );
}

fn remove_file_assocs() {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for id in &["kadr.image", "kadr.video"] {
        let _ = hkcu.delete_subkey_all(&format!("Software\\Classes\\{id}"));
    }
    for ext in &[
        "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "avif", "mp4", "mkv", "webm",
        "avi", "mov", "wmv", "flv", "m4v",
    ] {
        let path = format!("Software\\Classes\\.{ext}");
        if let Ok(key) = hkcu.open_subkey(&path) {
            let val: std::result::Result<String, _> = key.get_value("");
            let is_kadr = val.as_deref().map(|v| v == "kadr.image" || v == "kadr.video").unwrap_or(false);
            if is_kadr {
                let _ = hkcu.delete_subkey_all(&path);
            }
        }
    }
}

fn delete_install_dir(dir: &Path) -> Result<()> {
    // The installer exe itself may be running from here. Use cmd /c rd to schedule deletion.
    // First try direct removal.
    if std::fs::remove_dir_all(dir).is_ok() {
        return Ok(());
    }

    // Fallback: schedule removal on next reboot via MoveFileEx (MOVEFILE_DELAY_UNTIL_REBOOT)
    // Use a bat-script launched detached instead for simplicity
    let bat = std::env::temp_dir().join("kadr_cleanup.bat");
    let content = format!(
        "@echo off\r\ntimeout /t 2 /nobreak >nul\r\nrmdir /s /q \"{}\"\r\ndel \"%~f0\"\r\n",
        dir.display()
    );
    std::fs::write(&bat, content).context("Cannot write cleanup bat")?;
    std::process::Command::new("cmd")
        .args(["/c", "start", "/min", "", bat.to_str().unwrap_or("")])
        .spawn()
        .context("Cannot spawn cleanup bat")?;
    Ok(())
}
