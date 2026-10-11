use anyhow::Result;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type Baseline = BTreeMap<String, String>;

static DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_dir(dir: PathBuf) {
    let _ = DIR.set(dir);
}

fn file_for(pack_dir: &Path, instance: &Path) -> Option<PathBuf> {
    let mut hasher = Sha1::new();
    hasher.update(pack_dir.to_string_lossy().as_bytes());
    hasher.update(b"\n");
    hasher.update(instance.to_string_lossy().as_bytes());
    let name = format!("{}.json", hex::encode(hasher.finalize()));
    DIR.get().map(|dir| dir.join(name))
}

pub fn load(pack_dir: &Path, instance: &Path) -> Baseline {
    file_for(pack_dir, instance)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(pack_dir: &Path, instance: &Path, baseline: &Baseline) -> Result<()> {
    let Some(path) = file_for(pack_dir, instance) else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string(baseline)?)?;
    Ok(())
}

pub fn forget(pack_dir: &Path, instance: &Path) -> Result<()> {
    if let Some(path) = file_for(pack_dir, instance) {
        crate::util::delete_file(&path)?;
    }
    Ok(())
}
