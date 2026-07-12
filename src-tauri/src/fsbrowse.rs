use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::PathBuf;

const MAX_TEXT: u64 = 2_000_000;

const HIDDEN: &[&str] = &[
    ".git",
    ".ds_store",
    ".spotlight-v100",
    ".trashes",
    ".fseventsd",
    ".localized",
    ".appledouble",
    "thumbs.db",
    "desktop.ini",
    "__macosx",
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    pub text: Option<String>,
    pub binary: bool,
    pub too_large: bool,
    pub size: u64,
}

fn resolve(root: &str, rel: &str) -> Result<PathBuf> {
    if rel.split(['/', '\\']).any(|c| c == "..") {
        return Err(anyhow!("path escapes the root: {rel}"));
    }
    let mut p = PathBuf::from(root);
    if !rel.is_empty() {
        p.push(rel);
    }
    Ok(p)
}

pub fn list_dir(root: &str, rel: &str) -> Result<Vec<FsEntry>> {
    let dir = resolve(root, rel)?;
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if HIDDEN.contains(&name.to_lowercase().as_str()) {
            continue;
        }
        let meta = entry.metadata().ok();
        out.push(FsEntry {
            name,
            is_dir: meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
        });
    }
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

pub fn read_file(root: &str, rel: &str) -> Result<FileContent> {
    let p = resolve(root, rel)?;
    let size = std::fs::metadata(&p)?.len();
    if size > MAX_TEXT {
        return Ok(FileContent {
            text: None,
            binary: false,
            too_large: true,
            size,
        });
    }
    match String::from_utf8(std::fs::read(&p)?) {
        Ok(text) => Ok(FileContent {
            text: Some(text),
            binary: false,
            too_large: false,
            size,
        }),
        Err(_) => Ok(FileContent {
            text: None,
            binary: true,
            too_large: false,
            size,
        }),
    }
}

pub fn read_image(root: &str, rel: &str) -> Result<String> {
    use base64::Engine;
    let p = resolve(root, rel)?;
    let bytes = std::fs::read(&p)?;
    let mime = match p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

pub fn read_nbt(root: &str, rel: &str) -> Result<crate::nbt::NbtNode> {
    let p = resolve(root, rel)?;
    let size = std::fs::metadata(&p)?.len();
    if size > 64 * 1024 * 1024 {
        return Err(anyhow!("file too large to parse"));
    }
    crate::nbt::parse(&std::fs::read(&p)?)
}

pub fn write_file(root: &str, rel: &str, content: &str) -> Result<()> {
    let p = resolve(root, rel)?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, content)?;
    Ok(())
}

pub fn create_directory(root: &str, rel: &str) -> Result<()> {
    std::fs::create_dir_all(resolve(root, rel)?)?;
    Ok(())
}

pub fn delete_entry(root: &str, rel: &str) -> Result<()> {
    let p = resolve(root, rel)?;
    if p.is_dir() {
        std::fs::remove_dir_all(p)?;
    } else if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

pub fn rename_entry(root: &str, from: &str, to: &str) -> Result<()> {
    let f = resolve(root, from)?;
    let t = resolve(root, to)?;
    if let Some(parent) = t.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(f, t)?;
    Ok(())
}
