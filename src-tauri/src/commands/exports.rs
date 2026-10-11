use std::path::PathBuf;
use tauri::State;

use crate::cfpack;
use crate::curseforge::CurseForge;
use crate::dist;
use crate::instance;
use crate::lockfile;
use crate::manifest;
use crate::modrinth::Modrinth;
use crate::mrpack;
use crate::packignore::ExportFilter;
use crate::ptype::Folders;
use crate::publish;
use crate::secrets;

use super::{blocking, dist_token, es, resolve_loader_version};

#[tauri::command]
pub async fn export_mrpack(
    mr: State<'_, Modrinth>,
    path: String,
    output: String,
    env: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let filter = ExportFilter::load(&dir);
    let mut lock = lockfile::read(&dir).map_err(es)?;
    lock.mods.retain(|m| !m.disabled && !filter.skips_mod(m));
    let manifest = manifest::read(&dir).map_err(es)?;
    lock.loader_version = resolve_loader_version(&mr, &manifest).await;
    mrpack::export(
        &lock,
        &manifest.name,
        &manifest.version,
        &PathBuf::from(output),
        &env,
        &filter.overrides(&dir),
        &Folders::of(&manifest),
    )
    .await
    .map_err(es)
}

#[tauri::command]
pub async fn export_curseforge(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    output: String,
    env: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let filter = ExportFilter::load(&dir);
    let mut lock = lockfile::read(&dir).map_err(es)?;
    lock.mods.retain(|m| !m.disabled && !filter.skips_mod(m));
    let manifest = manifest::read(&dir).map_err(es)?;
    lock.loader_version = resolve_loader_version(&mr, &manifest).await;
    cfpack::export(
        &cf,
        &lock,
        &manifest.name,
        &manifest.version,
        &PathBuf::from(output),
        &env,
        &filter.overrides(&dir),
        &Folders::of(&manifest),
    )
    .await
    .map_err(es)
}

#[tauri::command]
pub async fn export_instance(
    mr: State<'_, Modrinth>,
    path: String,
    output: String,
    env: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    let lv = resolve_loader_version(&mr, &manifest).await;
    dist::export_instance(&dir, &PathBuf::from(output), &env, lv.as_deref())
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn release_folder() -> Result<String, String> {
    super::blocking(|| {
        let root = std::env::temp_dir().join("packweave-release");
        for entry in std::fs::read_dir(&root).into_iter().flatten().flatten() {
            let stale = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age.as_secs() > 3600);
            if stale {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
        let folder = root.join(crate::cache::now().to_string());
        std::fs::create_dir_all(&folder)?;
        Ok(folder.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn release_checkout(path: String, commit: String) -> Result<String, String> {
    blocking(move || {
        crate::worktree::checkout_commit(&PathBuf::from(&path), &commit)
            .map(|folder| folder.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn release_checkout_remove(path: String, folder: String) -> Result<(), String> {
    blocking(move || crate::worktree::remove_build(&PathBuf::from(&path), &PathBuf::from(&folder)))
        .await
}

#[tauri::command]
pub async fn include_packweaver(path: String) -> Result<(), String> {
    let _held = super::lock_pack(&PathBuf::from(&path)).await;
    blocking(move || dist::include_installer(&PathBuf::from(&path))).await
}

#[tauri::command]
pub async fn export_mrpack_selfupdate(
    mr: State<'_, Modrinth>,
    path: String,
    output: String,
    url: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let mut manifest = manifest::read(&dir).map_err(es)?;
    manifest.loader_version = resolve_loader_version(&mr, &manifest).await;
    let overrides = instance::overrides_dir(&dir);
    let token = dist_token(&dir, &url);
    mrpack::export_selfupdate(
        &manifest,
        &PathBuf::from(output),
        &url,
        &overrides,
        dist::INSTALLER,
        dist::INSTALLER_JAR,
        token.as_deref(),
    )
    .map_err(es)
}

#[tauri::command]
pub async fn export_curseforge_selfupdate(
    mr: State<'_, Modrinth>,
    path: String,
    output: String,
    url: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let mut manifest = manifest::read(&dir).map_err(es)?;
    manifest.loader_version = resolve_loader_version(&mr, &manifest).await;
    let token = dist_token(&dir, &url);
    cfpack::export_selfupdate(
        &manifest,
        &PathBuf::from(output),
        &url,
        dist::INSTALLER,
        dist::INSTALLER_JAR,
        token.as_deref(),
    )
    .map_err(es)
}

#[tauri::command]
pub async fn export_dist(
    mr: State<'_, Modrinth>,
    path: String,
    output: String,
    pack_url: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    let lv = resolve_loader_version(&mr, &manifest).await;
    let token = dist_token(&dir, &pack_url);
    dist::export_dist(
        &dir,
        &PathBuf::from(output),
        &pack_url,
        lv.as_deref(),
        token.as_deref(),
    )
    .map_err(es)
}

#[tauri::command]
pub async fn publish_pack(
    cf: State<'_, CurseForge>,
    target: String,
    path: String,
    project_id: String,
    version_number: String,
    changelog: String,
    env: String,
) -> Result<String, String> {
    let dir = PathBuf::from(&path);
    let filter = ExportFilter::load(&dir);
    let mut lock = lockfile::read(&dir).map_err(es)?;
    lock.mods.retain(|m| !m.disabled && !filter.skips_mod(m));
    let manifest = manifest::read(&dir).map_err(es)?;
    let overrides = filter.overrides(&dir);
    match target.as_str() {
        "curseforge" => {
            let token = secrets::get("curseforge_token").unwrap_or_default();
            publish::publish_curseforge(
                &cf,
                &lock,
                &manifest,
                &overrides,
                &project_id,
                &version_number,
                &changelog,
                &env,
                &token,
            )
            .await
            .map_err(es)
        }
        _ => {
            let token = secrets::get("modrinth_token").unwrap_or_default();
            publish::publish_modrinth(
                &lock,
                &manifest,
                &overrides,
                &project_id,
                &version_number,
                &changelog,
                &env,
                &token,
            )
            .await
            .map_err(es)
        }
    }
}
