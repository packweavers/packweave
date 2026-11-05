use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};
use ts_rs::TS;

use crate::manifest::Manifest;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ConvertInfo {
    pub format: String,
    pub name: String,
    pub content: usize,
    pub files: usize,
    pub git: bool,
}

#[derive(Serialize, Default, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Converted {
    pub from: String,
    pub content: usize,
    pub files: usize,
    pub unlocked: usize,
    pub disabled: Vec<String>,
    pub excluded: Vec<String>,
    pub skipped: Vec<String>,
    pub kept_source: bool,
}

pub fn root_of(path: &Path) -> Option<PathBuf> {
    crate::packwiz::root_of(path).or_else(|| crate::pakku::root_of(path))
}

pub fn convertible(dir: &Path) -> bool {
    crate::packwiz::is_packwiz(dir) || crate::pakku::is_pakku(dir)
}

pub fn info(dir: &Path) -> Result<Option<ConvertInfo>> {
    if crate::packwiz::is_packwiz(dir) {
        return crate::packwiz::info(dir).map(Some);
    }
    if crate::pakku::is_pakku(dir) {
        return crate::pakku::info(dir).map(Some);
    }
    Ok(None)
}

pub fn convert(src: &Path, dest: &Path) -> Result<(Manifest, Converted)> {
    if crate::packwiz::root_of(src).is_some() {
        crate::packwiz::convert(src, dest)
    } else {
        crate::pakku::convert(src, dest)
    }
}

pub(crate) fn place(from: &Path, to: &Path, moving: bool) -> Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if moving && std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    if moving {
        std::fs::remove_file(from)?;
    }
    Ok(())
}

pub(crate) fn remove_empty_dirs(root: &Path, start: &Path) {
    let mut dir = start.to_path_buf();
    while dir.starts_with(root) && dir != root {
        if std::fs::remove_dir(&dir).is_err() {
            break;
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break,
        }
    }
}

pub(crate) fn copy_docs(src: &Path, dest: &Path, skipped: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(src)?.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        let doc = lower.ends_with(".md") || lower.starts_with("license");
        if path.is_file() && doc && !dest.join(&name).exists() {
            if let Err(e) = std::fs::copy(&path, dest.join(&name)) {
                skipped.push(format!("{name}: couldn't copy it ({e})"));
            }
        }
    }
    Ok(())
}
