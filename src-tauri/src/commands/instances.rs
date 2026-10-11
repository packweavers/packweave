use std::path::{Path, PathBuf};
use tauri::State;

use crate::curseforge::CurseForge;
use crate::instance;
use crate::launch::{self, LogChunk, Running};
use crate::launchers::{self, DetectedInstance};
use crate::lockfile;
use crate::modrinth::Modrinth;
use crate::packignore::{self, IgnoreKind};
use crate::packlocal;
use crate::sync::{self, SyncOp, SyncReport};
use crate::syncignore::{self, Ignored};
use crate::undo::{self, Snapshot};

use super::{blocking, es, lock_pack, resolve_held, PackResolved};
use ts_rs::TS;

fn bound_instance(dir: &Path) -> Result<PathBuf, String> {
    packlocal::read(dir)
        .instance_dir
        .map(PathBuf::from)
        .ok_or_else(|| "No instance linked".to_string())
}

#[tauri::command]
pub async fn bind_instance(path: String, instance: String) -> Result<DetectedInstance, String> {
    let dir = PathBuf::from(&path);
    let inst = PathBuf::from(&instance);
    let pack_real = crate::util::real_path(&dir);
    let inst_real = crate::util::real_path(&inst);
    if pack_real == inst_real
        || pack_real.starts_with(&inst_real)
        || inst_real.starts_with(&pack_real)
    {
        return Err(
            "The pack folder and the instance folder can't be the same or inside each other."
                .into(),
        );
    }
    instance::ensure_gitignore(&dir).map_err(es)?;
    let mut local = packlocal::read(&dir);
    local.instance_dir = Some(instance);
    packlocal::write(&dir, &local).map_err(es)?;
    Ok(describe(&inst))
}

#[tauri::command]
pub async fn unbind_instance(path: String) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let mut local = packlocal::read(&dir);
    if let Some(old) = local.instance_dir.take() {
        crate::syncbase::forget(&dir, Path::new(&old)).map_err(es)?;
    }
    packlocal::write(&dir, &local).map_err(es)
}

fn describe(game_dir: &Path) -> DetectedInstance {
    let mut found = launchers::resolve_folder(game_dir);
    found.game_dir = game_dir.to_string_lossy().to_string();
    found
}

#[tauri::command]
pub async fn get_binding(path: String) -> Result<Option<DetectedInstance>, String> {
    Ok(packlocal::read(Path::new(&path))
        .instance_dir
        .map(|game_dir| describe(Path::new(&game_dir))))
}

#[tauri::command]
pub async fn sync_status(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
) -> Result<SyncReport, String> {
    sync::status(&mr, &cf, Path::new(&path)).await.map_err(es)
}

#[tauri::command]
pub async fn apply_sync(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    ops: Vec<SyncOp>,
    label: String,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let instance = bound_instance(&dir)?;
    if launch::running(&instance).is_some() {
        return Err("Minecraft is running in this instance. Stop it first.".into());
    }
    let guard = lock_pack(&dir).await;
    let lock = lockfile::read(&dir).ok();
    let mut snapshot = Snapshot::pack(&dir, label);
    for file in sync::affected_paths(&dir, &ops, lock.as_ref()) {
        snapshot = snapshot.file(&file);
    }
    let result = async {
        sync::apply_pull(&dir, &ops)?;
        let resolved = resolve_held(&mr, &cf, &dir, &guard).await?;
        sync::apply_push(&mr, &dir, &ops, &resolved.lockfile).await?;
        Ok::<_, anyhow::Error>(resolved)
    }
    .await;
    undo::record(&dir, snapshot);
    result.map_err(es)
}

#[tauri::command]
pub async fn sync_ignore(path: String, items: Vec<Ignored>) -> Result<Vec<Ignored>, String> {
    let instance = bound_instance(Path::new(&path))?;
    blocking(move || syncignore::add(&instance, items)).await
}

#[tauri::command]
pub async fn sync_unignore(path: String, keys: Vec<String>) -> Result<Vec<Ignored>, String> {
    let instance = bound_instance(Path::new(&path))?;
    blocking(move || syncignore::remove(&instance, &keys)).await
}

#[derive(serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct InstanceJar {
    pub file: String,
    pub enabled: bool,
}

#[tauri::command]
pub async fn instance_jars(path: String) -> Result<Vec<InstanceJar>, String> {
    let instance = bound_instance(Path::new(&path))?;
    blocking(move || {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(instance.join("mods")) else {
            return Ok(out);
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            if lower.ends_with(".jar") {
                out.push(InstanceJar {
                    file: name,
                    enabled: true,
                });
            } else if lower.ends_with(".jar.disabled") {
                out.push(InstanceJar {
                    file: name[..name.len() - ".disabled".len()].to_string(),
                    enabled: false,
                });
            }
        }
        out.sort_by_key(|j| j.file.to_lowercase());
        Ok(out)
    })
    .await
}

#[tauri::command]
pub async fn set_instance_jars(
    path: String,
    enable: Vec<String>,
    disable: Vec<String>,
) -> Result<(), String> {
    let instance = bound_instance(Path::new(&path))?;
    if launch::running(&instance).is_some() {
        return Err("Minecraft is running in this instance. Close it first.".into());
    }
    blocking(move || {
        let mods = instance.join("mods");
        for file in enable.iter().filter(|f| crate::pathsafe::is_plain_name(f)) {
            let off = mods.join(format!("{file}.disabled"));
            let on = mods.join(file);
            if off.is_file() && !on.exists() {
                std::fs::rename(off, on)?;
            }
        }
        for file in disable.iter().filter(|f| crate::pathsafe::is_plain_name(f)) {
            let on = mods.join(file);
            let off = mods.join(format!("{file}.disabled"));
            if on.is_file() && !off.exists() {
                std::fs::rename(on, off)?;
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn sync_file_diff(path: String, rel: String, kind: String) -> Result<String, String> {
    sync::file_diff(Path::new(&path), &rel, &kind).map_err(es)
}

#[tauri::command]
pub async fn auto_push_file(
    path: String,
    rel: String,
    original: Option<String>,
) -> Result<bool, String> {
    sync::auto_push_file(Path::new(&path), &rel, original.as_deref()).map_err(es)
}

#[tauri::command]
pub async fn read_ignore(path: String, kind: IgnoreKind) -> Result<String, String> {
    Ok(packignore::read(Path::new(&path), kind))
}

#[tauri::command]
pub async fn default_ignore(kind: IgnoreKind) -> Result<String, String> {
    Ok(kind.defaults().to_string())
}

#[tauri::command]
pub async fn write_ignore(path: String, kind: IgnoreKind, content: String) -> Result<(), String> {
    packignore::write(Path::new(&path), kind, &content).map_err(es)
}

#[tauri::command]
pub async fn add_ignore_patterns(
    path: String,
    kind: IgnoreKind,
    patterns: Vec<String>,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    for pattern in patterns.iter().filter(|p| !p.trim().is_empty()) {
        packignore::add_pattern(&dir, kind, pattern.trim()).map_err(es)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn detect_instances() -> Result<Vec<DetectedInstance>, String> {
    blocking(|| Ok(launchers::detect())).await
}

#[tauri::command]
pub async fn resolve_instance_folder(path: String) -> Result<DetectedInstance, String> {
    Ok(launchers::resolve_folder(Path::new(&path)))
}

#[tauri::command]
pub async fn instance_running(path: String) -> Result<Option<Running>, String> {
    let Ok(instance) = bound_instance(Path::new(&path)) else {
        return Ok(None);
    };
    blocking(move || Ok(launch::running(&instance))).await
}

#[tauri::command]
pub async fn read_game_log(path: String, offset: u64) -> Result<LogChunk, String> {
    let instance = bound_instance(Path::new(&path))?;
    launch::read_log(&instance, offset).map_err(es)
}

#[tauri::command]
pub async fn create_prism_instance(path: String) -> Result<DetectedInstance, String> {
    blocking(move || {
        let manifest = crate::manifest::read(Path::new(&path))?;
        launch::create_prism_instance(&manifest.name, &manifest)
    })
    .await
}

#[tauri::command]
pub async fn delete_instance(game_dir: String) -> Result<(), String> {
    blocking(move || launch::delete_instance(Path::new(&game_dir))).await
}

#[tauri::command]
pub async fn can_copy_instance(game_dir: String) -> Result<bool, String> {
    Ok(launch::can_copy(Path::new(&game_dir)))
}

#[tauri::command]
pub async fn copy_instance(game_dir: String, name: String) -> Result<DetectedInstance, String> {
    blocking(move || launch::copy_instance(Path::new(&game_dir), &name)).await
}

fn icon_path(dir: &Path) -> PathBuf {
    instance::overrides_dir(dir).join("icon.png")
}

#[tauri::command]
pub async fn pack_icon(path: String) -> Result<Option<String>, String> {
    use base64::Engine;
    match std::fs::read(icon_path(Path::new(&path))) {
        Ok(bytes) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Ok(Some(format!("data:image/png;base64,{b64}")))
        }
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub async fn set_pack_icon(path: String, source: Option<String>) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let icon = icon_path(&dir);
    let label = if source.is_some() {
        "Change pack icon"
    } else {
        "Remove pack icon"
    };
    let snapshot = Snapshot::new(label).file(&icon);
    match source {
        Some(src) => {
            std::fs::create_dir_all(instance::overrides_dir(&dir)).map_err(es)?;
            std::fs::copy(&src, &icon).map_err(es)?;
        }
        None => match std::fs::remove_file(&icon) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(es(e)),
            _ => {}
        },
    }
    undo::record(&dir, snapshot);
    Ok(())
}
