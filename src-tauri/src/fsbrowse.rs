use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use ts_rs::TS;

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

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Serialize, Default, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct FileContent {
    pub text: Option<String>,
    pub missing: bool,
    pub binary: bool,
    pub too_large: bool,
    pub size: u64,
}

pub fn resolve(root: &str, rel: &str) -> Result<PathBuf> {
    use std::path::Component;
    if rel.split(['/', '\\']).any(|c| c == "..")
        || rel.starts_with(['/', '\\'])
        || std::path::Path::new(rel).components().any(|c| {
            matches!(
                c,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        })
    {
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
    let size = match std::fs::metadata(&p) {
        Ok(meta) => meta.len(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FileContent {
                missing: true,
                ..Default::default()
            });
        }
        Err(e) => return Err(e.into()),
    };
    if size > MAX_TEXT {
        return Ok(FileContent {
            too_large: true,
            size,
            ..Default::default()
        });
    }
    match String::from_utf8(std::fs::read(&p)?) {
        Ok(text) => Ok(FileContent {
            text: Some(text),
            size,
            ..Default::default()
        }),
        Err(_) => Ok(FileContent {
            binary: true,
            size,
            ..Default::default()
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

pub fn trash(path: &Path) -> Result<()> {
    trash_context().delete(path)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn trash_context() -> trash::TrashContext {
    use trash::macos::{DeleteMethod, TrashContextExtMacos};
    let mut context = trash::TrashContext::default();
    context.set_delete_method(DeleteMethod::NsFileManager);
    context
}

#[cfg(not(target_os = "macos"))]
fn trash_context() -> trash::TrashContext {
    trash::TrashContext::default()
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
