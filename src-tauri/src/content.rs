use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::instance;
use crate::lockfile::{DepType, LockedMod, SourceFile};
use crate::manifest::Manifest;
use crate::providers::ProviderId;
use crate::ptype::ProjectType;

pub const CONTENT_DIR: &str = "content";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentItem {
    #[serde(default)]
    pub name: String,
    #[serde(skip)]
    pub project_type: ProjectType,
    #[serde(default = "auto_side")]
    pub side: String,
    #[serde(default)]
    pub explicit: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub preferred: ProviderId,
    #[serde(default)]
    pub sources: BTreeMap<ProviderId, SourceFile>,
    #[serde(default)]
    pub dependents: Vec<String>,
    #[serde(default = "required")]
    pub client_side: String,
    #[serde(default = "required")]
    pub server_side: String,
}

impl ContentItem {
    pub fn active(&self) -> Option<&SourceFile> {
        self.sources.get(&self.preferred)
    }
    pub fn project_id(&self) -> String {
        self.active()
            .and_then(|s| s.id.clone())
            .filter(|x| !x.is_empty())
            .or_else(|| {
                self.active()
                    .map(|s| s.slug.clone())
                    .filter(|x| !x.is_empty())
            })
            .unwrap_or_else(|| file_slug(self))
    }
}

fn auto_side() -> String {
    "auto".into()
}
fn required() -> String {
    "required".into()
}

pub fn type_dir(project_type: ProjectType) -> &'static str {
    project_type.content_dir()
}

fn type_from_dir(sub: &str) -> ProjectType {
    ProjectType::from_content_dir(sub)
}

pub fn content_dir(pack_dir: &Path) -> PathBuf {
    pack_dir.join(CONTENT_DIR)
}

fn item_path(
    pack_dir: &Path,
    project_type: ProjectType,
    slug: &str,
) -> PathBuf {
    content_dir(pack_dir)
        .join(type_dir(project_type))
        .join(format!("{slug}.json"))
}

pub fn file_slug(item: &ContentItem) -> String {
    let pref_slug = item.active().map(|s| s.slug.as_str()).unwrap_or("");
    let base = if !pref_slug.is_empty() {
        pref_slug
    } else {
        &item.name
    };
    let s: String = base
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if s.is_empty() {
        "item".into()
    } else {
        s
    }
}

pub fn matches(item: &ContentItem, key: &str) -> bool {
    item.sources
        .values()
        .any(|s| s.id.as_deref() == Some(key) || s.url.as_deref() == Some(key))
        || item.active().map(|s| s.slug == key).unwrap_or(false)
        || file_slug(item) == key
}

pub fn read_all(pack_dir: &Path) -> Vec<ContentItem> {
    let mut items = Vec::new();
    for sub in ["mods", "resourcepacks", "shaders"] {
        let dir = content_dir(pack_dir).join(sub);
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(mut item) = serde_json::from_str::<ContentItem>(&text)
                {
                    item.project_type = type_from_dir(sub);
                    items.push(item);
                }
            }
        }
    }
    items.sort_by_key(|i| i.name.to_lowercase());
    items
}

pub fn authored(pack_dir: &Path) -> Vec<ContentItem> {
    read_all(pack_dir)
        .into_iter()
        .filter(|i| i.explicit && !i.disabled)
        .collect()
}

pub fn add_item(pack_dir: &Path, item: &ContentItem) -> Result<()> {
    let path = item_path(pack_dir, item.project_type, &file_slug(item));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&committed_form(item))?),
    )?;
    Ok(())
}

fn committed_form(item: &ContentItem) -> ContentItem {
    let mut c = item.clone();
    for src in c.sources.values_mut() {
        if src.pin.is_none() {
            src.version_id.clear();
            src.version_number.clear();
            src.filename.clear();
            src.download_url.clear();
            src.sha1 = None;
            src.sha512 = None;
            src.file_size = 0;
        }
    }
    c
}

pub fn remove_item(pack_dir: &Path, key: &str) -> Result<()> {
    for item in read_all(pack_dir) {
        if matches(&item, key) {
            let p = item_path(pack_dir, item.project_type, &file_slug(&item));
            let _ = std::fs::remove_file(p);
        }
    }
    Ok(())
}

pub fn set_explicit(
    pack_dir: &Path,
    key: &str,
    explicit: bool,
) -> Result<bool> {
    for mut item in read_all(pack_dir) {
        if matches(&item, key) {
            item.explicit = explicit;
            add_item(pack_dir, &item)?;
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn set_disabled(
    pack_dir: &Path,
    key: &str,
    disabled: bool,
) -> Result<bool> {
    for mut item in read_all(pack_dir) {
        if matches(&item, key) {
            item.disabled = disabled;
            add_item(pack_dir, &item)?;
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn set_pin(
    pack_dir: &Path,
    key: &str,
    pin: Option<String>,
) -> Result<bool> {
    for mut item in read_all(pack_dir) {
        if matches(&item, key) {
            let pref = item.preferred;
            if let Some(s) = item.sources.get_mut(&pref) {
                s.pin = pin;
            }
            item.explicit = true;
            add_item(pack_dir, &item)?;
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn add_alt(
    pack_dir: &Path,
    key: &str,
    provider: ProviderId,
    id: &str,
) -> Result<bool> {
    for mut item in read_all(pack_dir) {
        if matches(&item, key) {
            if item.sources.contains_key(&provider) {
                return Ok(false);
            }
            item.sources.insert(
                provider,
                SourceFile {
                    id: Some(id.into()),
                    ..Default::default()
                },
            );
            item.explicit = true;
            add_item(pack_dir, &item)?;
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn remove_alt(
    pack_dir: &Path,
    key: &str,
    provider: ProviderId,
) -> Result<bool> {
    for mut item in read_all(pack_dir) {
        if matches(&item, key) {
            if provider == item.preferred {
                return Ok(false);
            }
            if item.sources.remove(&provider).is_none() {
                return Ok(false);
            }
            add_item(pack_dir, &item)?;
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn set_preferred(
    pack_dir: &Path,
    key: &str,
    provider: ProviderId,
) -> Result<bool> {
    for mut item in read_all(pack_dir) {
        if matches(&item, key) {
            if !item.sources.contains_key(&provider) {
                return Ok(false);
            }
            item.preferred = provider;
            item.explicit = true;
            add_item(pack_dir, &item)?;
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn write_resolved(
    pack_dir: &Path,
    manifest: &Manifest,
    resolved: &[ContentItem],
) -> Result<()> {
    let base = content_dir(pack_dir);
    for sub in ["mods", "resourcepacks", "shaders"] {
        let _ = std::fs::remove_dir_all(base.join(sub));
    }
    for item in resolved {
        add_item(pack_dir, item)?;
    }
    write_index(pack_dir)?;
    write_readme(pack_dir, manifest, resolved)?;
    Ok(())
}

fn collect_files(base: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, out);
        } else if path.is_file() {
            let rel = match path.strip_prefix(base) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if instance::is_os_junk(&rel) {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                let mut hasher = Sha1::new();
                hasher.update(&bytes);
                out.push((rel, hex::encode(hasher.finalize())));
            }
        }
    }
}

fn write_index(pack_dir: &Path) -> Result<()> {
    let mut files: Vec<(String, String)> = Vec::new();
    collect_files(pack_dir, &content_dir(pack_dir), &mut files);
    collect_files(pack_dir, &instance::overrides_dir(pack_dir), &mut files);
    let disabled: std::collections::HashSet<String> = read_all(pack_dir)
        .iter()
        .filter(|i| i.disabled)
        .map(|i| {
            format!(
                "content/{}/{}.json",
                type_dir(i.project_type),
                file_slug(i)
            )
        })
        .collect();
    files.retain(|(path, _)| !disabled.contains(path));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let arr: Vec<serde_json::Value> = files
        .into_iter()
        .map(|(path, sha1)| serde_json::json!({ "path": path, "sha1": sha1 }))
        .collect();
    std::fs::write(
        pack_dir.join("index.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({ "files": arr }))?
        ),
    )?;
    Ok(())
}

fn write_readme(
    pack_dir: &Path,
    manifest: &Manifest,
    resolved: &[ContentItem],
) -> Result<()> {
    let path = pack_dir.join("README.md");
    let count = |ty: ProjectType, explicit: bool| {
        resolved
            .iter()
            .filter(|i| i.project_type == ty && i.explicit == explicit)
            .count()
    };
    let mods = count(ProjectType::Mod, true);
    let deps = count(ProjectType::Mod, false);
    let rps = count(ProjectType::Resourcepack, true)
        + count(ProjectType::Resourcepack, false);
    let shaders =
        count(ProjectType::Shader, true) + count(ProjectType::Shader, false);
    let noun =
        |n: usize, s: &str, p: &str| (if n == 1 { s } else { p }).to_string();
    let mut segs = vec![if deps > 0 {
        format!(
            "{mods} {} ({deps} {})",
            noun(mods, "mod", "mods"),
            noun(deps, "dependency", "dependencies")
        )
    } else {
        format!("{mods} {}", noun(mods, "mod", "mods"))
    }];
    if rps > 0 {
        segs.push(format!(
            "{rps} {}",
            noun(rps, "resource pack", "resource packs")
        ));
    }
    if shaders > 0 {
        segs.push(format!("{shaders} {}", noun(shaders, "shader", "shaders")));
    }
    let loader = match &manifest.loader_version {
        Some(v) => format!("{} {}", manifest.loader, v),
        None => manifest.loader.clone(),
    };
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", manifest.name));
    md.push_str(&format!(
        "**Minecraft {} · {}**, {}\n\n",
        manifest.minecraft,
        loader,
        segs.join(", ")
    ));
    md.push_str("## Contents\n\n");
    let mut rows: Vec<&ContentItem> =
        resolved.iter().filter(|i| i.explicit).collect();
    rows.sort_by_key(|i| i.name.to_lowercase());
    for item in rows {
        let pref = item.active();
        let version = if pref.map(|s| s.pin.is_some()).unwrap_or(false) {
            pref.map(|s| s.version_number.clone()).unwrap_or_default()
        } else {
            String::new()
        };
        let slug = pref.map(|s| s.slug.clone()).unwrap_or_default();
        let link = if item.preferred == ProviderId::Modrinth {
            format!("https://modrinth.com/project/{slug}")
        } else {
            String::new()
        };
        let vsuffix = if version.is_empty() {
            String::new()
        } else {
            format!(" `{version}`")
        };
        if link.is_empty() {
            md.push_str(&format!(
                "- {}{} ({})\n",
                item.name, vsuffix, item.preferred
            ));
        } else {
            md.push_str(&format!("- [{}]({}){}\n", item.name, link, vsuffix));
        }
    }
    md.push_str("\n_Generated by packweave._\n");
    std::fs::write(&path, md)?;
    Ok(())
}

pub fn to_locked(item: &ContentItem) -> LockedMod {
    LockedMod {
        name: item.name.clone(),
        project_id: item.project_id(),
        slug: item.active().map(|s| s.slug.clone()).unwrap_or_default(),
        project_type: item.project_type,
        preferred: item.preferred,
        sources: item.sources.clone(),
        dependency_type: if item.explicit {
            DepType::Explicit
        } else {
            DepType::Dependency
        },
        dependents: item.dependents.clone(),
        client_side: item.client_side.clone(),
        server_side: item.server_side.clone(),
        disabled: item.disabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::SourceFile;

    fn item(pin: Option<String>) -> ContentItem {
        let mut item = ContentItem {
            preferred: ProviderId::Modrinth,
            ..Default::default()
        };
        item.sources.insert(
            ProviderId::Modrinth,
            SourceFile {
                id: Some("emi".into()),
                slug: "emi".into(),
                version_id: "abc".into(),
                version_number: "1.0".into(),
                filename: "emi.jar".into(),
                download_url: "https://example/emi.jar".into(),
                sha1: Some("hash".into()),
                file_size: 100,
                pin,
                ..Default::default()
            },
        );
        item
    }

    #[test]
    fn unpinned_committed_form_is_version_agnostic() {
        let c = committed_form(&item(None));
        let s = c.sources.get(&ProviderId::Modrinth).unwrap();
        assert!(s.version_id.is_empty());
        assert!(s.download_url.is_empty());
        assert!(s.filename.is_empty());
        assert!(s.sha1.is_none());
        assert_eq!(s.file_size, 0);
        assert_eq!(s.slug, "emi");
        assert_eq!(s.id.as_deref(), Some("emi"));
    }

    #[test]
    fn pinned_committed_form_keeps_version() {
        let c = committed_form(&item(Some("abc".into())));
        let s = c.sources.get(&ProviderId::Modrinth).unwrap();
        assert_eq!(s.version_id, "abc");
        assert_eq!(s.download_url, "https://example/emi.jar");
        assert_eq!(s.file_size, 100);
    }

    #[test]
    fn file_slug_sanitizes() {
        let mut it = ContentItem {
            name: "Some Mod!".into(),
            ..Default::default()
        };
        assert_eq!(file_slug(&it), "some-mod-");
        it.preferred = ProviderId::Modrinth;
        it.sources.insert(
            ProviderId::Modrinth,
            SourceFile {
                slug: "sodium".into(),
                ..Default::default()
            },
        );
        assert_eq!(file_slug(&it), "sodium");
    }
}
