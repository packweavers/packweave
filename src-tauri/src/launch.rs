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
        bail!("Minecraft is running in this instance. Stop it first.");
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
        .ok_or_else(|| anyhow!("can't find the instances folder"))?;
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
