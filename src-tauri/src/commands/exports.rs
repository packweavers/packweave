use std::path::PathBuf;
use tauri::State;

use crate::cfpack;
use crate::dist;
use crate::download::{self, DownloadReport};
use crate::instance;
use crate::lockfile;
use crate::manifest;
use crate::modrinth::Modrinth;
use crate::mrpack;
use crate::publish;
use crate::secrets;

use super::{es, resolve_loader_version};

#[tauri::command]
pub async fn export_mrpack(
    state: State<'_, Modrinth>,
    path: String,
    output: String,
    env: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let mut lock = lockfile::read(&dir).map_err(es)?;
    lock.mods.retain(|m| !m.disabled);
    let manifest = manifest::read(&dir).map_err(es)?;
    lock.loader_version =
        resolve_loader_version(state.inner(), &manifest).await;
    let overrides = instance::overrides_dir(&dir);
    mrpack::export(
        &lock,
        &manifest.name,
        &manifest.version,
        &PathBuf::from(output),
        &env,
        &overrides,
    )
    .map_err(es)
}

#[tauri::command]
pub async fn export_curseforge(
    state: State<'_, Modrinth>,
    path: String,
    output: String,
    env: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let mut lock = lockfile::read(&dir).map_err(es)?;
    lock.mods.retain(|m| !m.disabled);
    let manifest = manifest::read(&dir).map_err(es)?;
    lock.loader_version =
        resolve_loader_version(state.inner(), &manifest).await;
    let overrides = instance::overrides_dir(&dir);
    cfpack::export(
        &lock,
        &manifest.name,
        &manifest.version,
        &PathBuf::from(output),
        &env,
        &overrides,
    )
    .await
    .map_err(es)
}

#[tauri::command]
pub async fn export_instance(
    state: State<'_, Modrinth>,
    path: String,
    output: String,
    env: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    let lv = resolve_loader_version(state.inner(), &manifest).await;
    dist::export_instance(&dir, &PathBuf::from(output), &env, lv.as_deref())
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn export_mrpack_selfupdate(
    state: State<'_, Modrinth>,
    path: String,
    output: String,
    url: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let mut manifest = manifest::read(&dir).map_err(es)?;
    manifest.loader_version =
        resolve_loader_version(state.inner(), &manifest).await;
    mrpack::export_selfupdate(
        &manifest,
        &PathBuf::from(output),
        &url,
        dist::INSTALLER,
        dist::INSTALLER_JAR,
    )
    .map_err(es)
}

#[tauri::command]
pub async fn export_curseforge_selfupdate(
    state: State<'_, Modrinth>,
    path: String,
    output: String,
    url: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let mut manifest = manifest::read(&dir).map_err(es)?;
    manifest.loader_version =
        resolve_loader_version(state.inner(), &manifest).await;
    cfpack::export_selfupdate(
        &manifest,
        &PathBuf::from(output),
        &url,
        dist::INSTALLER,
        dist::INSTALLER_JAR,
    )
    .map_err(es)
}

#[tauri::command]
pub async fn download_mods(
    state: State<'_, Modrinth>,
    path: String,
    output: String,
) -> Result<DownloadReport, String> {
    let dir = PathBuf::from(&path);
    let mut lock = lockfile::read(&dir).map_err(es)?;
    lock.mods.retain(|m| !m.disabled);
    download::download_mods(state.inner(), &lock, &PathBuf::from(output))
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn export_dist(
    state: State<'_, Modrinth>,
    path: String,
    output: String,
    pack_url: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    let lv = resolve_loader_version(state.inner(), &manifest).await;
    dist::export_dist(&dir, &PathBuf::from(output), &pack_url, lv.as_deref())
        .map_err(es)
}

#[tauri::command]
pub async fn publish_pack(
    target: String,
    path: String,
    project_id: String,
    version_number: String,
    changelog: String,
    env: String,
) -> Result<String, String> {
    let dir = PathBuf::from(&path);
    let mut lock = lockfile::read(&dir).map_err(es)?;
    lock.mods.retain(|m| !m.disabled);
    let manifest = manifest::read(&dir).map_err(es)?;
    let overrides = instance::overrides_dir(&dir);
    match target.as_str() {
        "curseforge" => {
            let token = secrets::get("curseforge_token").unwrap_or_default();
            publish::publish_curseforge(
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
