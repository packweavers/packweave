use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::launchers::{self, DetectedInstance};
use ts_rs::TS;

#[derive(Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Running {
    pub pid: u32,
    pub started: u64,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct LogChunk {
    pub text: String,
    pub offset: u64,
    pub reset: bool,
}

fn processes() -> System {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always),
    );
    system
}

fn find(system: &System, game_dir: &Path) -> Option<Pid> {
    let game_dir = crate::util::real_path(game_dir);
    let game = game_dir.to_string_lossy().to_string();
    let inside = format!("{game}{}", std::path::MAIN_SEPARATOR);
    let assigned = format!("={game}");
    let root = launchers::mmc_root(&game_dir)
        .filter(|r| r != &game_dir)
        .map(|r| format!("{}{}", r.to_string_lossy(), std::path::MAIN_SEPARATOR));
    system
        .processes()
        .iter()
        .find(|(_, process)| {
            let name = process.name().to_string_lossy().to_lowercase();
            if !name.contains("java") {
                return false;
            }
            if process.cwd() == Some(game_dir.as_path()) {
                return true;
            }
            process.cmd().iter().any(|arg| {
                let arg = arg.to_string_lossy();
                arg == game
                    || arg.ends_with(&assigned)
                    || arg.contains(&inside)
                    || root.as_deref().is_some_and(|r| arg.contains(r))
            })
        })
        .map(|(pid, _)| *pid)
}

pub fn running(game_dir: &Path) -> Option<Running> {
    let system = processes();
    let pid = find(&system, game_dir)?;
    let process = system.process(pid)?;
    Some(Running {
        pid: pid.as_u32(),
        started: process.start_time(),
    })
}

pub fn read_log(game_dir: &Path, offset: u64) -> Result<LogChunk> {
    let path = game_dir.join("logs").join("latest.log");
    let mut file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => {
            return Ok(LogChunk {
                text: String::new(),
                offset: 0,
                reset: offset > 0,
            })
        }
    };
    let len = file.metadata()?.len();
    let (start, reset) = if len < offset {
        (0, true)
    } else {
        (offset, false)
    };
    let start = start.max(len.saturating_sub(512 * 1024));
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(LogChunk {
        text: String::from_utf8_lossy(&bytes).into_owned(),
        offset: start + bytes.len() as u64,
        reset: reset || (offset == 0 && start > 0),
    })
}

const SKIP_WHEN_COPYING: &[&str] = &["logs", "crash-reports", ".cache", "debug", "screenshots"];

fn copy_tree(src: &Path, dst: &Path, depth: usize) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if depth <= 1 && SKIP_WHEN_COPYING.contains(&name.as_str()) {
            continue;
        }
        let target = dst.join(&name);
        if path.is_dir() {
            copy_tree(&path, &target, depth + 1)?;
        } else if path.is_file() {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

fn folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        "instance".into()
    } else {
        cleaned
    }
}

pub fn delete_instance(game_dir: &Path) -> Result<()> {
    let root = launchers::mmc_root(game_dir)
        .ok_or_else(|| anyhow!("Only Prism Launcher and MultiMC instances can be deleted here."))?;
    if running(game_dir).is_some() {
        bail!("Minecraft is running in this instance. Close it first.");
    }
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

pub fn can_copy(game_dir: &Path) -> bool {
    launchers::mmc_root(game_dir).is_some()
}

pub fn copy_instance(game_dir: &Path, name: &str) -> Result<DetectedInstance> {
    let root = launchers::mmc_root(game_dir)
        .ok_or_else(|| anyhow!("Only Prism Launcher and MultiMC instances can be copied."))?;
    let instances = root
        .parent()
        .ok_or_else(|| anyhow!("Couldn't find the instances folder."))?;
    let base = folder_name(name);
    let mut target = instances.join(&base);
    let mut n = 2;
    while target.exists() {
        target = instances.join(format!("{base} {n}"));
        n += 1;
    }
    if let Err(e) = copy_tree(&root, &target, 0) {
        let _ = std::fs::remove_dir_all(&target);
        return Err(e);
    }
    let cfg_path = target.join("instance.cfg");
    let cfg = std::fs::read_to_string(&cfg_path).unwrap_or_default();
    let mut lines: Vec<String> = cfg
        .lines()
        .filter(|l| !l.starts_with("name=") && !l.starts_with("ManagedPack"))
        .map(str::to_string)
        .collect();
    lines.push(format!("name={}", name.trim()));
    std::fs::write(&cfg_path, format!("{}\n", lines.join("\n")))?;
    Ok(launchers::resolve_folder(&target))
}

pub fn create_prism_instance(
    name: &str,
    manifest: &crate::manifest::Manifest,
) -> Result<DetectedInstance> {
    let instances = launchers::prism_instances_dir()
        .ok_or_else(|| anyhow!("Prism Launcher isn't installed."))?;
    std::fs::create_dir_all(&instances)?;
    let base = folder_name(name);
    let mut root = instances.join(&base);
    let mut n = 2;
    while root.exists() {
        root = instances.join(format!("{base} {n}"));
        n += 1;
    }
    std::fs::create_dir_all(root.join(".minecraft"))?;
    std::fs::write(
        root.join("instance.cfg"),
        format!("InstanceType=OneSix\nname={}\n", name.trim()),
    )?;
    std::fs::write(root.join("mmc-pack.json"), crate::dist::mmc_pack(manifest)?)?;
    Ok(launchers::resolve_folder(&root))
}

const PRISM_NOT_FOUND: &str = "Prism Launcher isn't installed or couldn't be found.";

fn on_path(exe: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(exe).is_file()))
}

fn find_prism(data_dir: &Path) -> Option<std::process::Command> {
    use std::process::Command;
    let exe = if cfg!(windows) {
        "prismlauncher.exe"
    } else {
        "prismlauncher"
    };
    // Portable installs keep the executable next to the instances folder.
    let portable = data_dir.join(exe);
    if portable.is_file() {
        return Some(Command::new(portable));
    }
    if data_dir
        .to_string_lossy()
        .contains("org.prismlauncher.PrismLauncher")
    {
        let flatpak = on_path("flatpak").then(|| {
            let mut cmd = Command::new("flatpak");
            cmd.args(["run", "org.prismlauncher.PrismLauncher"]);
            cmd
        });
        return flatpak;
    }
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if cfg!(windows) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(Path::new(&local).join("Programs/PrismLauncher").join(exe));
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            candidates.push(
                Path::new(&home)
                    .join("scoop/apps/prismlauncher/current")
                    .join(exe),
            );
        }
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            candidates.push(Path::new(&pf).join("PrismLauncher").join(exe));
        }
    } else if cfg!(target_os = "macos") {
        candidates.push("/Applications/Prism Launcher.app/Contents/MacOS/prismlauncher".into());
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(
                Path::new(&home)
                    .join("Applications/Prism Launcher.app/Contents/MacOS/prismlauncher"),
            );
        }
    }
    if let Some(found) = candidates.into_iter().find(|p| p.is_file()) {
        return Some(Command::new(found));
    }
    // Linux packages, snap and AppImage symlinks.
    on_path(exe).then(|| Command::new(exe))
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct LaunchCheck {
    /// The linked folder belongs to a Prism/MultiMC-style instance.
    pub available: bool,
    /// Why it can't be launched right now, if anything.
    pub problem: Option<String>,
}

#[derive(Debug)]
struct Launchable {
    id: std::ffi::OsString,
    data_dir: std::path::PathBuf,
}

fn check_instance(root: &Path) -> Result<Launchable, String> {
    if !root.join("instance.cfg").is_file() {
        return Err("The instance has no instance.cfg.".into());
    }
    let pack: serde_json::Value = std::fs::read_to_string(root.join("mmc-pack.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .ok_or("The instance has no readable mmc-pack.json.")?;
    let has_minecraft = pack["components"]
        .as_array()
        .is_some_and(|c| c.iter().any(|c| c["uid"] == "net.minecraft"));
    if !has_minecraft {
        return Err("The instance has no Minecraft version set.".into());
    }
    let id = root
        .file_name()
        .ok_or("Couldn't read the instance folder name.")?;
    let data_dir = root
        .parent()
        .and_then(Path::parent)
        .ok_or("Couldn't find the Prism Launcher folder.")?;
    if !data_dir.join("prismlauncher.cfg").is_file() {
        return Err("The instance isn't in a Prism Launcher instances folder.".into());
    }
    Ok(Launchable {
        id: id.to_os_string(),
        data_dir: data_dir.to_path_buf(),
    })
}

pub fn launch_check(game_dir: &Path) -> LaunchCheck {
    let Some(root) = launchers::mmc_root(game_dir) else {
        return LaunchCheck {
            available: false,
            problem: None,
        };
    };
    let problem = match check_instance(&root) {
        Err(problem) => Some(problem),
        Ok(found) if find_prism(&found.data_dir).is_none() => Some(PRISM_NOT_FOUND.into()),
        Ok(_) => None,
    };
    LaunchCheck {
        available: true,
        problem,
    }
}

pub fn launch_in_prism(game_dir: &Path) -> Result<()> {
    let root = launchers::mmc_root(game_dir)
        .ok_or_else(|| anyhow!("Only Prism Launcher instances can be launched from here."))?;
    if running(game_dir).is_some() {
        bail!("Minecraft is already running in this instance.");
    }
    let found = check_instance(&root).map_err(|e| anyhow!(e))?;
    let mut child = find_prism(&found.data_dir)
        .ok_or_else(|| anyhow!(PRISM_NOT_FOUND))?
        .arg("--launch")
        .arg(&found.id)
        .spawn()
        .map_err(|e| anyhow!("Couldn't start Prism Launcher: {e}"))?;
    // Reap the launcher when it exits so it doesn't linger as a zombie.
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_instance_requires_config_minecraft_and_prism_data_dir() {
        let data = std::env::temp_dir().join(format!("pw-launch-{}", std::process::id()));
        let root = data.join("instances").join("My Pack");
        std::fs::create_dir_all(&root).unwrap();

        assert!(check_instance(&root).unwrap_err().contains("instance.cfg"));
        std::fs::write(root.join("instance.cfg"), "name=My Pack\n").unwrap();
        assert!(check_instance(&root).unwrap_err().contains("mmc-pack.json"));
        std::fs::write(root.join("mmc-pack.json"), r#"{"components":[]}"#).unwrap();
        assert!(check_instance(&root)
            .unwrap_err()
            .contains("Minecraft version"));
        std::fs::write(
            root.join("mmc-pack.json"),
            r#"{"components":[{"uid":"net.minecraft","version":"1.21.1"}]}"#,
        )
        .unwrap();
        assert!(check_instance(&root)
            .unwrap_err()
            .contains("Prism Launcher instances"));
        std::fs::write(data.join("prismlauncher.cfg"), "").unwrap();
        let ok = check_instance(&root).unwrap();
        assert_eq!(ok.id, "My Pack");

        let _ = std::fs::remove_dir_all(&data);
    }
}
