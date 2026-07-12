use std::path::PathBuf;
use tauri::State;

use crate::curseforge::CurseForge;
use crate::instance;
use crate::launchers::{self, DetectedInstance};
use crate::modrinth::Modrinth;
use crate::packlocal;
use crate::sync::{self, SyncOp, SyncReport};

use super::{do_resolve, es, PackResolved};

#[tauri::command]
pub async fn bind_instance(
    path: String,
    instance: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let mut local = packlocal::read(&dir);
    local.instance_dir = Some(instance);
    packlocal::write(&dir, &local).map_err(es)
}

#[tauri::command]
pub async fn unbind_instance(path: String) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let mut local = packlocal::read(&dir);
    local.instance_dir = None;
    packlocal::write(&dir, &local).map_err(es)
}

#[tauri::command]
pub async fn get_binding(path: String) -> Result<Option<String>, String> {
    Ok(packlocal::read(&PathBuf::from(path)).instance_dir)
}

#[tauri::command]
pub async fn sync_status(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
) -> Result<SyncReport, String> {
    sync::status(state.inner(), cf.inner(), &PathBuf::from(path))
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn apply_sync(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    ops: Vec<SyncOp>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    sync::apply_pull(&dir, &ops).map_err(es)?;
    let resolved = do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)?;
    sync::apply_push(state.inner(), &dir, &ops, &resolved.lockfile)
        .await
        .map_err(es)?;
    Ok(resolved)
}

#[tauri::command]
pub async fn sync_file_diff(
    path: String,
    rel: String,
    kind: String,
) -> Result<String, String> {
    sync::file_diff(&PathBuf::from(path), &rel, &kind).map_err(es)
}

#[tauri::command]
pub async fn auto_push_file(
    path: String,
    rel: String,
    original: Option<String>,
) -> Result<bool, String> {
    sync::auto_push_file(&PathBuf::from(path), &rel, original.as_deref())
        .map_err(es)
}

#[tauri::command]
pub async fn read_gitignore(path: String) -> Result<String, String> {
    Ok(instance::read_gitignore(&PathBuf::from(path)))
}

#[tauri::command]
pub async fn write_gitignore(
    path: String,
    content: String,
) -> Result<(), String> {
    instance::write_gitignore(&PathBuf::from(path), &content).map_err(es)
}

#[tauri::command]
pub async fn detect_instances() -> Result<Vec<DetectedInstance>, String> {
    Ok(launchers::detect())
}

#[tauri::command]
pub async fn pack_icon(path: String) -> Result<Option<String>, String> {
    use base64::Engine;
    let p = instance::overrides_dir(&PathBuf::from(path)).join("icon.png");
    match std::fs::read(&p) {
        Ok(bytes) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Ok(Some(format!("data:image/png;base64,{b64}")))
        }
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub async fn set_pack_icon(path: String, source: String) -> Result<(), String> {
    let overrides = instance::overrides_dir(&PathBuf::from(path));
    std::fs::create_dir_all(&overrides).map_err(es)?;
    std::fs::copy(&source, overrides.join("icon.png")).map_err(es)?;
    Ok(())
}

#[tauri::command]
pub async fn clear_pack_icon(path: String) -> Result<(), String> {
    let p = instance::overrides_dir(&PathBuf::from(path)).join("icon.png");
    let _ = std::fs::remove_file(p);
    Ok(())
}
