use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::content::{self, ContentItem};
use crate::git;
use crate::manifest::{self, Manifest};
use crate::providers::ProviderId;
use crate::ptype::ProjectType;
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct PackDiff {
    pub items: Vec<ItemChange>,
    pub env: Option<EnvDiff>,
    pub files: Vec<FileChange>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ItemChange {
    pub kind: String,
    pub name: String,
    pub project_id: String,
    pub project_type: ProjectType,
    pub slug: String,
    pub path: String,
    pub dependency: bool,
    pub parents: Vec<String>,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub from_side: Option<String>,
    pub to_side: Option<String>,
    pub from_provider: Option<ProviderId>,
    pub to_provider: Option<ProviderId>,
    pub disabled: bool,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct EnvDiff {
    pub from_minecraft: String,
    pub to_minecraft: String,
    pub from_loader: String,
    pub to_loader: String,
    pub from_loader_version: Option<String>,
    pub to_loader_version: Option<String>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts", rename = "PackFileChange")]
pub struct FileChange {
    pub status: String,
    pub path: String,
}

struct SnapItem {
    name: String,
    project_id: String,
    path: String,
    explicit: bool,
    dependents: Vec<String>,
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
        project_id: item.project_id(),
        path: content::item_rel_path(item),
        explicit: item.explicit,
        dependents: item.dependents.clone(),
        project_type: item.project_type,
        slug: active
            .map(|s| s.slug.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback.to_string()),
        provider: item.preferred,
        version_id: active.map(|s| s.version_id.clone()).unwrap_or_default(),
        version_number: active.map(|s| s.version_number.clone()).unwrap_or_default(),
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
        item.project_type = ProjectType::from_content_path(&path);
        let slug = slug_from_path(&path);
        item.file = slug.clone();
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
    for sub in ProjectType::ALL.map(ProjectType::content_dir) {
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
            item.project_type = ProjectType::from_content_path(&rel);
            let slug = slug_from_path(&rel);
            item.file = slug.clone();
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
    names: &BTreeMap<String, String>,
) -> ItemChange {
    ItemChange {
        kind: kind.into(),
        name: repr.name.clone(),
        project_id: repr.project_id.clone(),
        project_type: repr.project_type,
        slug: repr.slug.clone(),
        path: repr.path.clone(),
        dependency: !repr.explicit,
        parents: repr
            .dependents
            .iter()
            .map(|id| names.get(id).cloned().unwrap_or_else(|| id.clone()))
            .collect(),
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

fn diff(from: &Snapshot, to: &Snapshot, from_known: bool) -> (Vec<ItemChange>, Option<EnvDiff>) {
    let mut items: Vec<ItemChange> = Vec::new();
    let names: BTreeMap<String, String> = from
        .items
        .values()
        .chain(to.items.values())
        .map(|i| (i.project_id.clone(), i.name.clone()))
        .collect();
    let keys: BTreeSet<&String> = from.items.keys().chain(to.items.keys()).collect();
    for key in keys {
        match (from.items.get(key), to.items.get(key)) {
            (None, Some(t)) => items.push(item_change("added", t, None, Some(t), &names)),
            (Some(f), None) => items.push(item_change("removed", f, Some(f), None, &names)),
            (Some(f), Some(t)) => {
                if f.disabled != t.disabled {
                    let kind = if t.disabled { "disabled" } else { "enabled" };
                    items.push(item_change(kind, t, Some(f), Some(t), &names));
                } else if !f.version_id.is_empty()
                    && !t.version_id.is_empty()
                    && f.version_id != t.version_id
                {
                    items.push(item_change("updated", t, Some(f), Some(t), &names));
                } else if f.provider != t.provider {
                    items.push(item_change("provider", t, Some(f), Some(t), &names));
                } else if f.side != t.side {
                    items.push(item_change("resided", t, Some(f), Some(t), &names));
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
