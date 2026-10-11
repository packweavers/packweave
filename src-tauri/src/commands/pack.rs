use std::path::{Path, PathBuf};
use tauri::State;

use crate::curseforge::CurseForge;
use crate::git;
use crate::import;
use crate::instance;
use crate::lockfile;
use crate::manifest::{self, Manifest};
use crate::modrinth::Modrinth;
use crate::packwiz;
use crate::ptype::Folders;
use crate::undo::{self, Snapshot};

use super::{blocking, do_resolve, es, PackState};

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
        datapacks: None,
        packweave_md: true,
        format_version: manifest::FORMAT_VERSION,
    };
    manifest::write(&dir, &manifest).map_err(es)?;
    instance::create_defaults(&dir).map_err(es)?;
    Ok(manifest)
}

#[tauri::command]
pub async fn open_pack(path: String) -> Result<PackState, String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    let warning = instance::ensure_defaults(&dir).err().map(es);
    let lockfile = lockfile::read(&dir).ok();
    Ok(PackState {
        dir: path,
        manifest,
        lockfile,
        converted: None,
        warning,
    })
}

#[tauri::command]
pub async fn read_manifest(path: String) -> Result<Manifest, String> {
    manifest::read(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn pin_loader_version(
    mr: State<'_, Modrinth>,
    path: String,
) -> Result<Option<Manifest>, String> {
    let dir = PathBuf::from(&path);
    let text = std::fs::read_to_string(manifest::path_for(&dir)).map_err(es)?;
    if crate::lenient::has_conflict(&text) {
        return Ok(None);
    }
    let mut manifest = manifest::read(&dir).map_err(es)?;
    let unset = manifest.loader_version.as_deref().is_none_or(str::is_empty);
    if manifest.loader == "vanilla" || !unset {
        return Ok(None);
    }
    let found = mr
        .loader_versions(&manifest.loader, &manifest.minecraft)
        .await
        .map_err(es)?;
    let Some(newest) = found.versions.into_iter().next() else {
        return Ok(None);
    };
    manifest.loader_version = Some(newest);
    manifest::write(&dir, &manifest).map_err(es)?;
    Ok(Some(manifest))
}

#[tauri::command]
pub async fn import_pack(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    src: String,
    dest: String,
) -> Result<PackState, String> {
    let dest_dir = PathBuf::from(&dest);
    let source = PathBuf::from(&src);
    let (manifest, converted) = match packwiz::root_of(&source) {
        Some(root) => {
            let target = dest_dir.clone();
            let (manifest, report) = blocking(move || packwiz::convert(&root, &target)).await?;
            (manifest, Some(report))
        }
        None => (
            import::import_pack(&mr, &cf, &source, &dest_dir)
                .await
                .map_err(es)?,
            None,
        ),
    };
    instance::ensure_defaults(&dest_dir).map_err(es)?;
    let _ = do_resolve(&mr, &cf, &dest_dir).await;
    Ok(PackState {
        dir: dest,
        manifest,
        lockfile: lockfile::read(&dest_dir).ok(),
        converted,
        warning: None,
    })
}

#[tauri::command]
pub async fn packwiz_info(path: String) -> Result<Option<packwiz::PackwizInfo>, String> {
    blocking(move || {
        let dir = PathBuf::from(&path);
        if !packwiz::is_packwiz(&dir) {
            return Ok(None);
        }
        packwiz::info(&dir).map(Some)
    })
    .await
}

#[tauri::command]
pub async fn convert_packwiz(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
) -> Result<PackState, String> {
    let dir = PathBuf::from(&path);
    let target = dir.clone();
    let (manifest, report) = blocking(move || packwiz::convert(&target, &target)).await?;
    instance::ensure_gitignore(&dir).map_err(es)?;
    instance::ensure_defaults(&dir).map_err(es)?;
    let _ = do_resolve(&mr, &cf, &dir).await;
    Ok(PackState {
        dir: path,
        manifest,
        lockfile: lockfile::read(&dir).ok(),
        converted: Some(report),
        warning: None,
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
    mr: State<'_, Modrinth>,
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
    let (manifest, converted) = match manifest::read(&dest) {
        Ok(m) => (m, None),
        Err(_) if packwiz::is_packwiz(&dest) => {
            let target = dest.clone();
            match blocking(move || packwiz::convert(&target, &target)).await {
                Ok((m, report)) => (m, Some(report)),
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&dest);
                    return Err(e);
                }
            }
        }
        Err(_) => {
            let _ = std::fs::remove_dir_all(&dest);
            return Err(
                "This repository isn't a packweave or packwiz pack (no modpack.json or pack.toml)."
                    .into(),
            );
        }
    };
    instance::ensure_gitignore(&dest).map_err(es)?;
    instance::ensure_defaults(&dest).map_err(es)?;
    let _ = do_resolve(&mr, &cf, &dest).await;
    Ok(PackState {
        dir: dest.to_string_lossy().to_string(),
        manifest,
        lockfile: lockfile::read(&dest).ok(),
        converted,
        warning: None,
    })
}

fn datapack_files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            name.ends_with(".zip") || name.ends_with(".zip.disabled")
        })
        .collect()
}

fn move_datapacks(root: &Path, from: &str, to: &str) -> anyhow::Result<()> {
    let source = root.join(from);
    let files = datapack_files(&source);
    if files.is_empty() {
        return Ok(());
    }
    let target = root.join(to);
    std::fs::create_dir_all(&target)?;
    for file in files {
        let Some(name) = file.file_name() else {
            continue;
        };
        let dest = target.join(name);
        if !dest.exists() {
            std::fs::rename(&file, &dest)?;
        }
    }
    let _ = std::fs::remove_dir(&source);
    Ok(())
}

#[tauri::command]
pub async fn set_datapack_folder(path: String, folder: String) -> Result<Manifest, String> {
    let folder = crate::ptype::datapack_folder(&folder).ok_or_else(|| {
        "Use a folder inside the game folder, like datapacks or config/paxi/datapacks. It can't be in mods, resourcepacks or shaderpacks."
            .to_string()
    })?;
    blocking(move || {
        let dir = PathBuf::from(&path);
        let mut manifest = manifest::read(&dir)?;
        let old = Folders::of(&manifest).datapacks().to_string();
        let overrides = instance::overrides_dir(&dir);
        let mut snapshot = Snapshot::pack(&dir, "Change the data pack folder");
        if old != folder {
            for file in datapack_files(&overrides.join(&old)) {
                snapshot = snapshot.file(&file);
                if let Some(name) = file.file_name() {
                    snapshot = snapshot.file(&overrides.join(&folder).join(name));
                }
            }
        }
        manifest.datapacks = Some(folder.clone());
        manifest::write(&dir, &manifest)?;
        if old != folder {
            move_datapacks(&overrides, &old, &folder)?;
            if let Some(game) = crate::packlocal::read(&dir).instance_dir {
                move_datapacks(Path::new(&game), &old, &folder)?;
            }
        }
        undo::record(&dir, snapshot);
        Ok(manifest)
    })
    .await
}

#[tauri::command]
pub async fn preserve_patterns(path: String) -> Result<Vec<String>, String> {
    Ok(crate::preserve::patterns(Path::new(&path)))
}

#[tauri::command]
pub async fn preserve_update(
    path: String,
    remove: Vec<String>,
    add: Vec<String>,
) -> Result<Vec<String>, String> {
    blocking(move || crate::preserve::update(Path::new(&path), &remove, &add)).await
}

#[tauri::command]
pub async fn preserve_reset(path: String) -> Result<Vec<String>, String> {
    blocking(move || crate::preserve::reset(Path::new(&path))).await
}

#[tauri::command]
pub async fn save_manifest(path: String, mut manifest: Manifest) -> Result<(), String> {
    let dir = PathBuf::from(path);
    if let Ok(existing) = manifest::read(&dir) {
        manifest.format_version = manifest.format_version.max(existing.format_version);
    }
    let snapshot = Snapshot::pack(&dir, "Change pack settings");
    manifest::write(&dir, &manifest).map_err(es)?;
    undo::record(&dir, snapshot);
    Ok(())
}
