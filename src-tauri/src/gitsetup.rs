use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::path::Path;
use std::process::Command;

use crate::git;
use ts_rs::TS;

pub const DOWNLOAD_URL: &str = "https://git-scm.com/downloads";

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct GitSetup {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub can_install: bool,
    pub install_label: Option<String>,
    pub manual_command: Option<String>,
    pub download_url: String,
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn exists(exe: &str) -> Option<std::path::PathBuf> {
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD".into())
            .split(';')
            .filter(|e| !e.is_empty())
            .map(|e| e.to_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let candidate = dir.join(format!("{exe}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn distro_command() -> Option<String> {
    let managers: [(&str, &str); 6] = [
        ("pacman", "sudo pacman -S --needed git"),
        ("apt", "sudo apt install git"),
        ("dnf", "sudo dnf install git"),
        ("zypper", "sudo zypper install git"),
        ("apk", "sudo apk add git"),
        ("xbps-install", "sudo xbps-install -S git"),
    ];
    managers
        .iter()
        .find(|(bin, _)| exists(bin).is_some())
        .map(|(_, cmd)| (*cmd).to_string())
}

fn version_of(exe: &Path) -> Option<String> {
    let mut cmd = Command::new(exe);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let v = text.trim().trim_start_matches("git version ").trim();
    (!v.is_empty()).then(|| v.to_string())
}

pub fn state() -> GitSetup {
    let bin = git::binary();
    let (name, email) = git::identity();

    #[cfg(target_os = "macos")]
    let (can_install, install_label, manual_command) = (
        true,
        Some("Install with Xcode Command Line Tools".to_string()),
        Some("xcode-select --install".to_string()),
    );

    #[cfg(target_os = "windows")]
    let (can_install, install_label, manual_command) = {
        let winget = exists("winget").is_some();
        (
            winget,
            winget.then(|| "Install with winget".to_string()),
            Some("winget install --id Git.Git -e".to_string()),
        )
    };

    #[cfg(target_os = "linux")]
    let (can_install, install_label, manual_command) = (false, None, distro_command());

    GitSetup {
        installed: bin.is_some(),
        version: bin.as_deref().and_then(version_of),
        path: bin.map(|p| p.to_string_lossy().to_string()),
        name,
        email,
        can_install,
        install_label,
        manual_command,
        download_url: DOWNLOAD_URL.to_string(),
    }
}

#[cfg(target_os = "macos")]
fn run_installer() -> Result<()> {
    if git::clt_installed() {
        bail!("The Xcode Command Line Tools are already installed.");
    }
    Command::new("/usr/bin/xcode-select")
        .arg("--install")
        .spawn()
        .map_err(|e| anyhow!("couldn't start the installer: {e}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_installer() -> Result<()> {
    let winget = exists("winget").ok_or_else(|| anyhow!("winget isn't available."))?;
    let mut cmd = Command::new(winget);
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000);
    let out = cmd
        .args([
            "install",
            "--id",
            "Git.Git",
            "-e",
            "--source",
            "winget",
            "--accept-source-agreements",
            "--accept-package-agreements",
        ])
        .output()
        .map_err(|e| anyhow!("couldn't start the installer: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = if err.trim().is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            err.trim().to_string()
        };
        bail!("the installer didn't finish: {err}");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_installer() -> Result<()> {
    bail!("Install git with your package manager, then choose Check again.")
}

pub fn install() -> Result<GitSetup> {
    run_installer()?;
    git::forget_binary();
    Ok(state())
}

pub fn recheck() -> GitSetup {
    git::forget_binary();
    state()
}
