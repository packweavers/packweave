use std::path::Path;

use crate::fsbrowse::{self, FileContent, FsEntry};
use crate::nbt::NbtNode;
use crate::search::{self, FileMatch, SearchOptions};
use crate::undo::{self, Snapshot};

use super::{blocking, es};

#[tauri::command]
pub async fn fs_list(root: String, rel: String) -> Result<Vec<FsEntry>, String> {
    fsbrowse::list_dir(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_read(root: String, rel: String) -> Result<FileContent, String> {
    fsbrowse::read_file(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_write(root: String, rel: String, content: String) -> Result<(), String> {
    fsbrowse::write_file(&root, &rel, &content).map_err(es)
}

#[tauri::command]
pub async fn fs_mkdir(root: String, rel: String) -> Result<(), String> {
    fsbrowse::create_directory(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_delete(root: String, rel: String) -> Result<(), String> {
    blocking(move || {
        let path = fsbrowse::resolve(&root, &rel)?;
        if !path.exists() {
            return Ok(());
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| rel.clone());
        let label = format!("Delete {name}");
        let snapshot = if path.is_dir() {
            Snapshot::new(label).dir(&path)
        } else {
            Snapshot::new(label).file(&path)
        };
        fsbrowse::trash(&path)?;
        undo::record(Path::new(&root), snapshot);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn fs_rename(root: String, from: String, to: String) -> Result<(), String> {
    fsbrowse::rename_entry(&root, &from, &to).map_err(es)
}

#[tauri::command]
pub async fn fs_read_image(root: String, rel: String) -> Result<String, String> {
    fsbrowse::read_image(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_read_nbt(root: String, rel: String) -> Result<NbtNode, String> {
    fsbrowse::read_nbt(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_index(root: String) -> Result<Vec<String>, String> {
    blocking(move || Ok(search::index_files(&root))).await
}

#[tauri::command]
pub async fn search_files(
    root: String,
    query: String,
    options: SearchOptions,
) -> Result<Vec<FileMatch>, String> {
    blocking(move || search::search_files(&root, &query, &options)).await
}

#[tauri::command]
pub async fn replace_in_files(
    root: String,
    query: String,
    options: SearchOptions,
    replacement: String,
    only: Vec<String>,
) -> Result<u32, String> {
    blocking(move || search::replace_in_files(&root, &query, &options, &replacement, &only)).await
}

#[tauri::command]
pub async fn mod_configs(path: String, key: String) -> Result<Vec<String>, String> {
    blocking(move || {
        Ok(crate::configs::for_content(
            std::path::Path::new(&path),
            &key,
        ))
    })
    .await
}
