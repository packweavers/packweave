use crate::fsbrowse::{self, FileContent, FsEntry};
use crate::nbt::NbtNode;
use crate::search::{self, FileMatch};

use super::es;

#[tauri::command]
pub async fn fs_list(
    root: String,
    rel: String,
) -> Result<Vec<FsEntry>, String> {
    fsbrowse::list_dir(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_read(root: String, rel: String) -> Result<FileContent, String> {
    fsbrowse::read_file(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_write(
    root: String,
    rel: String,
    content: String,
) -> Result<(), String> {
    fsbrowse::write_file(&root, &rel, &content).map_err(es)
}

#[tauri::command]
pub async fn fs_mkdir(root: String, rel: String) -> Result<(), String> {
    fsbrowse::create_directory(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_delete(root: String, rel: String) -> Result<(), String> {
    fsbrowse::delete_entry(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_rename(
    root: String,
    from: String,
    to: String,
) -> Result<(), String> {
    fsbrowse::rename_entry(&root, &from, &to).map_err(es)
}

#[tauri::command]
pub async fn fs_read_image(
    root: String,
    rel: String,
) -> Result<String, String> {
    fsbrowse::read_image(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn fs_read_nbt(root: String, rel: String) -> Result<NbtNode, String> {
    fsbrowse::read_nbt(&root, &rel).map_err(es)
}

#[tauri::command]
pub async fn search_files(
    root: String,
    query: String,
) -> Result<Vec<FileMatch>, String> {
    Ok(search::search_files(&root, &query))
}
