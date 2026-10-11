use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use ts_rs::TS;

static DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts", rename = "SyncIgnored")]
pub struct Ignored {
    pub key: String,
    pub label: String,
}

impl Ignored {
    pub fn covers(&self, key: &str) -> bool {
        self.key == key || (self.key.ends_with('/') && key.starts_with(&self.key))
    }
}

pub fn set_dir(dir: PathBuf) {
    let _ = DIR.set(dir);
}

fn file_for(instance: &Path) -> Option<PathBuf> {
    let mut hasher = Sha1::new();
    hasher.update(instance.to_string_lossy().as_bytes());
    let name = format!("{}.json", hex::encode(hasher.finalize()));
    DIR.get().map(|dir| dir.join(name))
}

pub fn load(instance: &Path) -> Vec<Ignored> {
    let Some(text) = file_for(instance).and_then(|p| std::fs::read_to_string(p).ok()) else {
        return Vec::new();
    };
    if let Ok(list) = serde_json::from_str(&text) {
        return list;
    }
    match crate::lenient::parse(&text) {
        Some((serde_json::Value::Array(items), _)) => items
            .into_iter()
            .filter_map(|item| serde_json::from_value(item).ok())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn covered(list: &[Ignored], key: &str) -> bool {
    list.iter().any(|i| i.covers(key))
}

pub fn add(instance: &Path, items: Vec<Ignored>) -> Result<Vec<Ignored>> {
    let mut list = load(instance);
    for item in items {
        if covered(&list, &item.key) {
            continue;
        }
        if item.key.ends_with('/') {
            list.retain(|i| !item.covers(&i.key));
        }
        list.push(item);
    }
    list.sort_by_key(|i| i.label.to_lowercase());
    save(instance, &list)?;
    Ok(list)
}

pub fn remove(instance: &Path, keys: &[String]) -> Result<Vec<Ignored>> {
    let mut list = load(instance);
    list.retain(|i| !keys.contains(&i.key));
    save(instance, &list)?;
    Ok(list)
}

pub fn rename(instance: &Path, moved: &[(String, String)]) -> Result<()> {
    let mut list = load(instance);
    let mut changed = false;
    for item in list.iter_mut() {
        if let Some((_, new)) = moved.iter().find(|(old, _)| item.key == *old) {
            item.key = new.clone();
            changed = true;
        }
    }
    if changed {
        save(instance, &list)?;
    }
    Ok(())
}

fn save(instance: &Path, list: &[Ignored]) -> Result<()> {
    let path = file_for(instance).ok_or_else(|| anyhow!("The app data folder isn't ready."))?;
    if list.is_empty() {
        crate::util::delete_file(&path)?;
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(list)?)?;
    Ok(())
}
