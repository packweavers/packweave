use std::path::PathBuf;
use tauri::State;

use crate::curseforge::CurseForge;
use crate::git;
use crate::import;
use crate::instance;
use crate::lockfile;
use crate::manifest::{self, Manifest};
use crate::modrinth::Modrinth;

use super::{do_resolve, es, PackState};

#[tauri::command]
pub async fn create_pack(
    path: String,
    name: String,
    minecraft: String,
    loader: String,
    loader_version: Option<String>,
) -> Result<Manifest, String> {
    let dir = PathBuf::from(&path);
    std::fs::create_dir_all(&dir).map_err(es)?;
    let manifest = Manifest {
        name,
        version: "1.0.0".into(),
        minecraft,
        loader,
        loader_version,
        channel: manifest::Channel::Release,
    };
    manifest::write(&dir, &manifest).map_err(es)?;
    instance::ensure_defaults(&dir);
    Ok(manifest)
}

#[tauri::command]
pub async fn open_pack(path: String) -> Result<PackState, String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    instance::ensure_defaults(&dir);
    let lockfile = lockfile::read(&dir).ok();
    Ok(PackState {
        dir: path,
        manifest,
        lockfile,
    })
}

#[tauri::command]
pub async fn import_pack(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    src: String,
    dest: String,
) -> Result<PackState, String> {
    let dest_dir = PathBuf::from(&dest);
    let manifest = import::import_pack(
        state.inner(),
        cf.inner(),
        &PathBuf::from(&src),
        &dest_dir,
    )
    .await
    .map_err(es)?;
    instance::ensure_defaults(&dest_dir);
    let _ = do_resolve(state.inner(), cf.inner(), &dest_dir).await;
    Ok(PackState {
        dir: dest,
        manifest,
        lockfile: lockfile::read(&dest_dir).ok(),
    })
}

fn repo_name(url: &str) -> String {
    let s = url.trim().trim_end_matches('/');
    let s = s.rsplit('/').next().unwrap_or(s);
    let s = s.strip_suffix(".git").unwrap_or(s);
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "pack".into()
    } else {
        cleaned
    }
}

#[tauri::command]
pub async fn clone_pack(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    url: String,
    dest_parent: String,
) -> Result<PackState, String> {
    let dest = PathBuf::from(&dest_parent).join(repo_name(&url));
    if dest.exists()
        && std::fs::read_dir(&dest)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        return Err(format!(
            "A folder named “{}” already exists here.",
            dest.file_name().and_then(|n| n.to_str()).unwrap_or("pack")
        ));
    }
    if let Err(e) = git::clone(&url, &dest) {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(es(e));
    }
    let manifest = match manifest::read(&dest) {
        Ok(m) => m,
        Err(_) => {
            let _ = std::fs::remove_dir_all(&dest);
            return Err(
                "This repository isn't a packweave pack (no modpack.json)."
                    .into(),
            );
        }
    };
    instance::ensure_defaults(&dest);
    let _ = do_resolve(state.inner(), cf.inner(), &dest).await;
    Ok(PackState {
        dir: dest.to_string_lossy().to_string(),
        manifest,
        lockfile: lockfile::read(&dest).ok(),
    })
}

#[tauri::command]
pub async fn save_manifest(
    path: String,
    manifest: Manifest,
) -> Result<(), String> {
    manifest::write(&PathBuf::from(path), &manifest).map_err(es)
}
