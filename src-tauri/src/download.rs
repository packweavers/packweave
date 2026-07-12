use anyhow::Result;
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::path::Path;

use crate::lockfile::{target_dir, Lockfile};
use crate::modrinth::Modrinth;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadReport {
    pub downloaded: u32,
    pub failed: Vec<String>,
    pub mods_dir: String,
}

pub async fn download_mods(
    mr: &Modrinth,
    lock: &Lockfile,
    out_dir: &Path,
) -> Result<DownloadReport> {
    let mut downloaded = 0u32;
    let mut failed: Vec<String> = Vec::new();

    for m in &lock.mods {
        let src = match m.active() {
            Some(s) if !s.download_url.is_empty() => s,
            _ => {
                failed.push(format!("{} (no download url)", m.name));
                continue;
            }
        };
        let bytes = match mr.download(&src.download_url).await {
            Ok(b) => b,
            Err(_) => {
                failed.push(format!("{} (download failed)", m.name));
                continue;
            }
        };
        if let Some(expected) = &src.sha1 {
            if &sha1_hex(&bytes) != expected {
                failed.push(format!("{} (hash mismatch)", m.name));
                continue;
            }
        }
        let dir = out_dir.join(target_dir(m.project_type));
        if std::fs::create_dir_all(&dir).is_err() {
            failed.push(format!("{} (write failed)", m.name));
            continue;
        }
        match std::fs::write(dir.join(&src.filename), &bytes) {
            Ok(_) => downloaded += 1,
            Err(_) => failed.push(format!("{} (write failed)", m.name)),
        }
    }

    Ok(DownloadReport {
        downloaded,
        failed,
        mods_dir: out_dir.to_string_lossy().to_string(),
    })
}

fn sha1_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
