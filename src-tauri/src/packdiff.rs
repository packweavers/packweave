use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::content::{self, ContentItem};
use crate::git;
use crate::manifest::{self, Manifest};
use crate::providers::ProviderId;
use crate::ptype::ProjectType;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackDiff {
    pub items: Vec<ItemChange>,
    pub env: Option<EnvDiff>,
    pub files: Vec<FileChange>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemChange {
    pub kind: String,
    pub name: String,
    pub project_type: ProjectType,
    pub slug: String,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub from_side: Option<String>,
    pub to_side: Option<String>,
    pub from_provider: Option<ProviderId>,
    pub to_provider: Option<ProviderId>,
    pub disabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvDiff {
    pub from_minecraft: String,
    pub to_minecraft: String,
    pub from_loader: String,
    pub to_loader: String,
    pub from_loader_version: Option<String>,
    pub to_loader_version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub status: String,
    pub path: String,
}

struct SnapItem {
    name: String,
    project_type: ProjectType,
    slug: String,
    provider: ProviderId,
    version_id: String,
    version_number: String,
    side: String,
    disabled: bool,
}

struct Snapshot {
    minecraft: String,
    loader: String,
    loader_version: Option<String>,
    items: BTreeMap<String, SnapItem>,
}

fn empty_snapshot() -> Snapshot {
    Snapshot {
        minecraft: String::new(),
        loader: String::new(),
        loader_version: None,
        items: BTreeMap::new(),
    }
}

fn type_from_path(path: &str) -> ProjectType {
    ProjectType::from_content_path(path)
}

fn slug_from_path(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .strip_suffix(".json")
        .unwrap_or("")
        .to_string()
}

fn snap_item(item: &ContentItem, fallback: &str) -> SnapItem {
    let active = item.active();
    SnapItem {
        name: if item.name.is_empty() {
            fallback.to_string()
        } else {
            item.name.clone()
        },
        project_type: item.project_type,
        slug: active
            .map(|s| s.slug.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback.to_string()),
        provider: item.preferred,
        version_id: active.map(|s| s.version_id.clone()).unwrap_or_default(),
        version_number: active
            .map(|s| s.version_number.clone())
            .unwrap_or_default(),
        side: item.side.clone(),
        disabled: item.disabled,
    }
}

fn snapshot_at(dir: &Path, reference: &str) -> Result<Snapshot> {
    let manifest: Manifest =
        serde_json::from_str(&git::show_file(dir, reference, "modpack.json")?)?;
    let mut items = BTreeMap::new();
    for path in git::list_tree(dir, reference, "content") {
        if !path.ends_with(".json") {
            continue;
        }
        let text = match git::show_file(dir, reference, &path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let mut item: ContentItem = match serde_json::from_str(&text) {
            Ok(i) => i,
            Err(_) => continue,
        };
        item.project_type = type_from_path(&path);
        let slug = slug_from_path(&path);
        let key = format!("{}/{}", item.project_type, slug);
        items.insert(key, snap_item(&item, &slug));
    }
    Ok(Snapshot {
        minecraft: manifest.minecraft,
        loader: manifest.loader,
        loader_version: manifest.loader_version,
        items,
    })
}

fn working_snapshot(dir: &Path) -> Snapshot {
    let (minecraft, loader, loader_version) = match manifest::read(dir) {
        Ok(m) => (m.minecraft, m.loader, m.loader_version),
        Err(_) => (String::new(), String::new(), None),
    };
    let mut items = BTreeMap::new();
    let base = content::content_dir(dir);
    for sub in ["mods", "resourcepacks", "shaders"] {
        let entries = match std::fs::read_dir(base.join(sub)) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let rel = format!(
                "content/{}/{}",
                sub,
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
            );
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let mut item: ContentItem = match serde_json::from_str(&text) {
                Ok(i) => i,
                Err(_) => continue,
            };
            item.project_type = type_from_path(&rel);
            let slug = slug_from_path(&rel);
            let key = format!("{}/{}", item.project_type, slug);
            items.insert(key, snap_item(&item, &slug));
        }
    }
    Snapshot {
        minecraft,
        loader,
        loader_version,
        items,
    }
}

fn item_change(
    kind: &str,
    repr: &SnapItem,
    from: Option<&SnapItem>,
    to: Option<&SnapItem>,
) -> ItemChange {
    ItemChange {
        kind: kind.into(),
        name: repr.name.clone(),
        project_type: repr.project_type,
        slug: repr.slug.clone(),
        from_version: from
            .map(|s| s.version_number.clone())
            .filter(|s| !s.is_empty()),
        to_version: to
            .map(|s| s.version_number.clone())
            .filter(|s| !s.is_empty()),
        from_side: from.map(|s| s.side.clone()),
        to_side: to.map(|s| s.side.clone()),
        from_provider: from.map(|s| s.provider),
        to_provider: to.map(|s| s.provider),
        disabled: repr.disabled,
    }
}

fn diff(
    from: &Snapshot,
    to: &Snapshot,
    from_known: bool,
) -> (Vec<ItemChange>, Option<EnvDiff>) {
    let mut items: Vec<ItemChange> = Vec::new();
    let keys: BTreeSet<&String> =
        from.items.keys().chain(to.items.keys()).collect();
    for key in keys {
        match (from.items.get(key), to.items.get(key)) {
            (None, Some(t)) => {
                items.push(item_change("added", t, None, Some(t)))
            }
            (Some(f), None) => {
                items.push(item_change("removed", f, Some(f), None))
            }
            (Some(f), Some(t)) => {
                if f.disabled != t.disabled {
                    let kind = if t.disabled { "disabled" } else { "enabled" };
                    items.push(item_change(kind, t, Some(f), Some(t)));
                } else if !f.version_id.is_empty()
                    && !t.version_id.is_empty()
                    && f.version_id != t.version_id
                {
                    items.push(item_change("updated", t, Some(f), Some(t)));
                } else if f.provider != t.provider {
                    items.push(item_change("provider", t, Some(f), Some(t)));
                } else if f.side != t.side {
                    items.push(item_change("resided", t, Some(f), Some(t)));
                }
            }
            (None, None) => {}
        }
    }
    items.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let env = if from_known
        && (from.minecraft != to.minecraft
            || from.loader != to.loader
            || from.loader_version != to.loader_version)
    {
        Some(EnvDiff {
            from_minecraft: from.minecraft.clone(),
            to_minecraft: to.minecraft.clone(),
            from_loader: from.loader.clone(),
            to_loader: to.loader.clone(),
            from_loader_version: from.loader_version.clone(),
            to_loader_version: to.loader_version.clone(),
        })
    } else {
        None
    };
    (items, env)
}

fn strip_overrides(p: &str) -> String {
    p.strip_prefix("overrides/").unwrap_or(p).to_string()
}

fn file_status(code: &str) -> &'static str {
    match code {
        "A" => "added",
        "D" => "removed",
        "R" => "renamed",
        "C" => "copied",
        _ => "modified",
    }
}

pub fn pack_diff(dir: &Path, from_ref: &str, to_ref: &str) -> Result<PackDiff> {
    let to = snapshot_at(dir, to_ref)?;
    let from_res = snapshot_at(dir, from_ref);
    let from_known = from_res.is_ok();
    let from = from_res.unwrap_or_else(|_| empty_snapshot());
    let (items, env) = diff(&from, &to, from_known);
    let files = git::name_status(dir, from_ref, to_ref, "overrides")
        .into_iter()
        .map(|(status, path)| FileChange {
            status: file_status(&status).into(),
            path: strip_overrides(&path),
        })
        .collect();
    Ok(PackDiff { items, env, files })
}

pub fn pack_diff_working(dir: &Path) -> Result<PackDiff> {
    let from_res = snapshot_at(dir, "HEAD");
    let from_known = from_res.is_ok();
    let from = from_res.unwrap_or_else(|_| empty_snapshot());
    let to = working_snapshot(dir);
    let (items, env) = diff(&from, &to, from_known);
    let files = git::name_status_working(dir, "overrides")
        .into_iter()
        .map(|(status, path)| FileChange {
            status: file_status(&status).into(),
            path: strip_overrides(&path),
        })
        .collect();
    Ok(PackDiff { items, env, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_overrides_prefix() {
        assert_eq!(
            strip_overrides("overrides/config/foo.json"),
            "config/foo.json"
        );
        assert_eq!(strip_overrides("config/foo.json"), "config/foo.json");
    }

    #[test]
    fn keys_by_path() {
        assert_eq!(slug_from_path("content/mods/emi.json"), "emi");
        assert_eq!(
            type_from_path("content/resourcepacks/x.json"),
            ProjectType::Resourcepack
        );
        assert_eq!(
            type_from_path("content/shaders/x.json"),
            ProjectType::Shader
        );
        assert_eq!(type_from_path("content/mods/x.json"), ProjectType::Mod);
    }

    #[test]
    fn file_status_words() {
        assert_eq!(file_status("A"), "added");
        assert_eq!(file_status("M"), "modified");
        assert_eq!(file_status("D"), "removed");
        assert_eq!(file_status("R"), "renamed");
    }

    fn snap(version_id: &str, provider: ProviderId) -> SnapItem {
        SnapItem {
            name: "EMI".into(),
            project_type: ProjectType::Mod,
            slug: "emi".into(),
            provider,
            version_id: version_id.into(),
            version_number: if version_id.is_empty() {
                String::new()
            } else {
                "1.0".into()
            },
            side: "auto".into(),
            disabled: false,
        }
    }

    #[test]
    fn disabling_a_mod_emits_a_disabled_change() {
        let mut from = snap("v1", ProviderId::Modrinth);
        let mut to = snap("v1", ProviderId::Modrinth);
        from.disabled = false;
        to.disabled = true;
        let (items, _) = diff(&snapshot_with(from), &snapshot_with(to), true);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, "disabled");
        assert!(items[0].disabled);
    }

    fn snapshot_with(item: SnapItem) -> Snapshot {
        let mut s = empty_snapshot();
        s.items.insert("mod/emi".into(), item);
        s
    }

    #[test]
    fn pin_toggle_is_not_a_change() {
        let pinned = snapshot_with(snap("abc", ProviderId::Modrinth));
        let unpinned = snapshot_with(snap("", ProviderId::Modrinth));
        assert!(diff(&pinned, &unpinned, true).0.is_empty());
        assert!(diff(&unpinned, &pinned, true).0.is_empty());
    }

    #[test]
    fn real_version_bump_still_updates() {
        let from = snapshot_with(snap("abc", ProviderId::Modrinth));
        let to = snapshot_with(snap("def", ProviderId::Modrinth));
        let items = diff(&from, &to, true).0;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, "updated");
    }

    #[test]
    fn provider_change_during_unpin_still_shows() {
        let from = snapshot_with(snap("abc", ProviderId::Modrinth));
        let to = snapshot_with(snap("", ProviderId::Curseforge));
        let items = diff(&from, &to, true).0;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, "provider");
    }
}
