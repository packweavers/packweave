use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::providers::ProviderId;
use crate::ptype::ProjectType;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DepType {
    Explicit,
    Dependency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lockfile {
    pub minecraft: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub mods: Vec<LockedMod>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    #[serde(default)]
    pub slug: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version_number: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub filename: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub download_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub file_size: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockedMod {
    pub name: String,
    pub project_id: String,
    pub slug: String,
    pub project_type: ProjectType,
    pub preferred: ProviderId,
    pub sources: BTreeMap<ProviderId, SourceFile>,
    pub dependency_type: DepType,
    pub dependents: Vec<String>,
    pub client_side: String,
    pub server_side: String,
    #[serde(default)]
    pub disabled: bool,
}

impl LockedMod {
    pub fn active(&self) -> Option<&SourceFile> {
        self.sources.get(&self.preferred)
    }
    pub fn download_url(&self) -> &str {
        self.active().map(|s| s.download_url.as_str()).unwrap_or("")
    }
    pub fn filename(&self) -> &str {
        self.active().map(|s| s.filename.as_str()).unwrap_or("")
    }
    pub fn sha1(&self) -> Option<&str> {
        self.active().and_then(|s| s.sha1.as_deref())
    }
    pub fn version_id(&self) -> &str {
        self.active().map(|s| s.version_id.as_str()).unwrap_or("")
    }
    pub fn version_number(&self) -> &str {
        self.active()
            .map(|s| s.version_number.as_str())
            .unwrap_or("")
    }
}

pub fn target_dir(project_type: ProjectType) -> &'static str {
    project_type.instance_dir()
}

static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_cache_dir(p: PathBuf) {
    let _ = CACHE_DIR.set(p);
}

fn cache_path(dir: &Path) -> Option<PathBuf> {
    let base = CACHE_DIR.get()?;
    let mut h = Sha1::new();
    h.update(dir.to_string_lossy().as_bytes());
    Some(base.join(format!("{}.json", hex::encode(h.finalize()))))
}

pub fn cache_put(dir: &Path, lock: &Lockfile) {
    if let Some(p) = cache_path(dir) {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = serde_json::to_string(lock) {
            let _ = std::fs::write(p, s);
        }
    }
}

fn cache_get(dir: &Path) -> Option<Lockfile> {
    let s = std::fs::read_to_string(cache_path(dir)?).ok()?;
    serde_json::from_str(&s).ok()
}

pub fn read(dir: &Path) -> Result<Lockfile> {
    let manifest = crate::manifest::read(dir)?;
    let items = crate::content::read_all(dir);
    let cached: HashMap<String, LockedMod> = cache_get(dir)
        .map(|c| {
            c.mods
                .into_iter()
                .map(|m| (m.project_id.clone(), m))
                .collect()
        })
        .unwrap_or_default();
    let mods = items
        .iter()
        .map(|it| {
            let mut m = crate::content::to_locked(it);
            if let Some(cm) = cached.get(&m.project_id) {
                for (prov, src) in m.sources.iter_mut() {
                    if src.download_url.is_empty() {
                        if let Some(cs) = cm.sources.get(prov) {
                            src.version_id = cs.version_id.clone();
                            src.version_number = cs.version_number.clone();
                            src.filename = cs.filename.clone();
                            src.download_url = cs.download_url.clone();
                            src.sha1 = cs.sha1.clone();
                            src.sha512 = cs.sha512.clone();
                            src.file_size = cs.file_size;
                            if src.slug.is_empty() {
                                src.slug = cs.slug.clone();
                            }
                        }
                    }
                }
            }
            m
        })
        .collect();
    Ok(Lockfile {
        minecraft: manifest.minecraft,
        loader: manifest.loader,
        loader_version: manifest.loader_version,
        mods,
    })
}
