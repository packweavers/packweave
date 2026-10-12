use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use tokio::sync::OwnedMutexGuard;

use crate::content;
use crate::curseforge::CurseForge;
use crate::lockfile::{self, Lockfile};
use crate::manifest::{self, Manifest};
use crate::modrinth::Modrinth;
use crate::resolver;
use crate::undo::{self, Snapshot};
use crate::validate::{self, Health, Validation};
use ts_rs::TS;

mod catalog;
mod content_ops;
mod exports;
mod files;
mod gitops;
mod history;
mod instances;
mod pack;
mod settings;
mod windows;

pub use catalog::*;
pub use content_ops::*;
pub use exports::*;
pub use files::*;
pub use gitops::*;
pub use history::*;
pub use instances::*;
pub use pack::*;
pub use settings::*;
pub use windows::*;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct PackState {
    pub dir: String,
    pub manifest: Manifest,
    pub lockfile: Option<Lockfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub converted: Option<crate::packwiz::Converted>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub warning: Option<String>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct PackResolved {
    pub manifest: Manifest,
    pub lockfile: Lockfile,
    pub validations: Vec<Validation>,
    pub health: Health,
    pub suggested: Vec<resolver::Suggested>,
}

pub(crate) fn es<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub(crate) fn dist_token(dir: &Path, update_url: &str) -> Option<String> {
    if !update_url.starts_with("release+https://github.com/") {
        return None;
    }
    saved_dist_token(dir)
}

pub(crate) fn saved_dist_token(dir: &Path) -> Option<String> {
    crate::secrets::get(&dist_token_key(dir)).filter(|t| !t.trim().is_empty())
}

pub(crate) fn dist_token_key(dir: &Path) -> String {
    format!(
        "pack_dist_token:{}",
        crate::worktree::main_dir(dir).to_string_lossy()
    )
}

pub(crate) async fn github_identity(dir: &Path) -> (Option<String>, Option<String>) {
    let dir = dir.to_path_buf();
    blocking(move || {
        Ok((
            crate::git::github_repo(&dir),
            crate::git::github_token(&dir),
        ))
    })
    .await
    .unwrap_or((None, None))
}

pub(crate) async fn blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(es)?
        .map_err(es)
}

pub(crate) type PackGuard = OwnedMutexGuard<()>;

fn pack_locks() -> &'static Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(Default::default)
}

pub(crate) async fn lock_pack(dir: &Path) -> PackGuard {
    let lock = pack_locks()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(crate::util::real_path(dir))
        .or_default()
        .clone();
    lock.lock_owned().await
}

pub(crate) async fn mutate<S, F>(
    mr: &Modrinth,
    cf: &CurseForge,
    dir: &Path,
    snapshot: S,
    change: F,
) -> Result<PackResolved, String>
where
    S: FnOnce(&Path) -> Snapshot,
    F: FnOnce(&Path) -> anyhow::Result<()>,
{
    let guard = lock_pack(dir).await;
    let snapshot = snapshot(dir);
    change(dir).map_err(es)?;
    undo::record(dir, snapshot);
    resolve_held(mr, cf, dir, &guard).await.map_err(es)
}

pub(crate) async fn resolve_loader_version(mr: &Modrinth, manifest: &Manifest) -> Option<String> {
    match manifest.loader_version.as_deref() {
        Some(v) if !v.is_empty() => Some(v.to_string()),
        _ if manifest.loader != "vanilla" => mr
            .loader_versions(&manifest.loader, &manifest.minecraft)
            .await
            .ok()
            .and_then(|v| v.versions.into_iter().next()),
        _ => None,
    }
}

pub(crate) fn known_items(
    items: &[content::ContentItem],
) -> std::collections::HashMap<String, content::ContentItem> {
    let mut map = std::collections::HashMap::new();
    for item in items {
        if let Some(id) = item
            .sources
            .get(&item.preferred)
            .and_then(|s| s.id.clone())
            .filter(|i| !i.is_empty())
        {
            map.insert(id, item.clone());
        }
    }
    map
}

pub(crate) async fn do_resolve(
    mr: &Modrinth,
    cf: &CurseForge,
    dir: &Path,
) -> anyhow::Result<PackResolved> {
    let guard = lock_pack(dir).await;
    resolve_held(mr, cf, dir, &guard).await
}

pub(crate) async fn resolve_held(
    mr: &Modrinth,
    cf: &CurseForge,
    dir: &Path,
    _held: &PackGuard,
) -> anyhow::Result<PackResolved> {
    let manifest = manifest::read(dir)?;
    let all =
        content::without_duplicates(content::merge_same_files(content::read_all_checked(dir)?));
    let authored: Vec<content::ContentItem> = all
        .iter()
        .filter(|i| i.explicit && !i.disabled)
        .cloned()
        .collect();
    let excluded: std::collections::HashSet<String> = all
        .iter()
        .filter(|i| i.disabled)
        .map(|i| i.project_id())
        .collect();
    let known = known_items(&all);
    let aliases = resolver::aliases(&all);
    let present: std::collections::HashSet<String> = all.iter().map(|i| i.project_id()).collect();
    let out = resolver::resolve(mr, cf, &manifest, &authored, &excluded, &known, &aliases).await?;
    let suggested = resolver::suggestions(&out, &present);
    let pinned: std::collections::HashSet<String> = all
        .iter()
        .filter(|i| i.pinned)
        .map(|i| i.project_id())
        .collect();
    let unbundled: std::collections::HashSet<String> = all
        .iter()
        .filter(|i| i.no_bundle)
        .map(|i| i.project_id())
        .collect();
    let mut combined = out.locked.clone();
    for item in combined.iter_mut() {
        item.pinned = pinned.contains(&item.project_id());
        item.no_bundle = unbundled.contains(&item.project_id());
    }
    let resolved: std::collections::HashSet<String> =
        out.locked.iter().map(|i| i.project_id()).collect();
    combined.extend(
        all.into_iter()
            .filter(|i| i.disabled && !resolved.contains(&i.project_id())),
    );
    let mut combined = content::merge_same_files(combined);
    content::assign_files(&mut combined);
    content::write_resolved(dir, &manifest, &combined)?;
    let lockfile = Lockfile {
        minecraft: manifest.minecraft.clone(),
        loader: manifest.loader.clone(),
        loader_version: resolve_loader_version(mr, &manifest).await,
        mods: combined.iter().map(content::to_locked).collect(),
    };
    lockfile::cache_put(dir, &lockfile);
    let local = validate::local_mods(dir);
    let (validations, health) = validate::build(&out, &lockfile, &local);
    Ok(PackResolved {
        manifest,
        lockfile,
        validations,
        health,
        suggested,
    })
}
