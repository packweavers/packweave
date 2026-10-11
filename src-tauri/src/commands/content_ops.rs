use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::bulkadd;
use crate::content::{self, ContentItem};
use crate::curseforge::CurseForge;
use crate::dropped;
use crate::instance;
use crate::lockfile::{self, SourceFile};
use crate::manifest;
use crate::modrinth::Modrinth;
use crate::providers::{self, ProviderId};
use crate::ptype::ProjectType;
use crate::resolver;
use crate::undo::Snapshot;

use super::{do_resolve, es, mutate, PackResolved};
use ts_rs::TS;

#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct AddItem {
    pub provider: ProviderId,
    pub project_id: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    pub slug: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    pub name: String,
    pub project_type: ProjectType,
    #[serde(default)]
    #[ts(optional)]
    pub url: Option<String>,
}

fn new_item(add: &AddItem) -> ContentItem {
    let display = [&add.name, &add.slug, &add.project_id]
        .into_iter()
        .find(|s| !s.is_empty())
        .cloned()
        .unwrap_or_default();
    let source = match add.provider {
        ProviderId::Url => SourceFile {
            url: add.url.clone().or_else(|| Some(add.project_id.clone())),
            slug: add.slug.clone(),
            ..Default::default()
        },
        _ => SourceFile {
            id: Some(add.project_id.clone()),
            slug: add.slug.clone(),
            ..Default::default()
        },
    };
    ContentItem {
        name: display,
        project_type: add.project_type,
        side: "auto".into(),
        explicit: true,
        preferred: add.provider,
        sources: BTreeMap::from([(add.provider, source)]),
        client_side: "required".into(),
        server_side: "required".into(),
        ..Default::default()
    }
}

fn add_key(add: &AddItem) -> String {
    match (add.provider, &add.url) {
        (ProviderId::Url, Some(url)) => url.clone(),
        _ => add.project_id.clone(),
    }
}

#[tauri::command]
pub async fn add_content(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    items: Vec<AddItem>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let existing = content::read_all(&dir);
    let fresh: Vec<&AddItem> = items
        .iter()
        .filter(|a| {
            !existing
                .iter()
                .any(|i| i.explicit && content::matches(i, &add_key(a)))
        })
        .collect();
    if fresh.is_empty() {
        return do_resolve(&mr, &cf, &dir).await.map_err(es);
    }
    let label = match fresh.as_slice() {
        [one] => format!("Add {}", new_item(one).name),
        many => format!("Add {} items", many.len()),
    };
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| {
            for add in &fresh {
                content::add_item(dir, &new_item(add))?;
            }
            Ok(())
        },
    )
    .await
}

#[tauri::command]
pub async fn bulk_lookup(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    provider: ProviderId,
    text: String,
) -> Result<bulkadd::BulkLookup, String> {
    let dir = PathBuf::from(&path);
    let existing: HashSet<String> = content::read_all(&dir)
        .iter()
        .filter(|i| i.explicit)
        .filter_map(|i| i.sources.get(&provider).and_then(|s| s.id.clone()))
        .collect();
    let manifest = manifest::read(&dir).map_err(es)?;
    let mr = mr.inner();
    let cf = cf.inner();
    let manifest = &manifest;
    let lookups = bulkadd::parse_refs(&text).into_iter().map(|r| async move {
        let failure = |r: &bulkadd::ParsedRef| bulkadd::BulkFailure {
            raw: r.raw.clone(),
            reason: format!("not found on {}", provider.label()),
            line: r.line,
        };
        let Some(p) = providers::by_id(provider, mr, cf) else {
            return Err(failure(&r));
        };
        if !r.by_name {
            if let Ok(project) = p.lookup(&r.reference).await {
                return Ok(bulkadd::BulkCandidate {
                    provider,
                    project_type: r.project_type.unwrap_or(project.project_type),
                    project_id: project.id,
                    slug: project.slug,
                    name: project.name,
                    icon_url: project.icon_url,
                    raw: r.raw.clone(),
                    line: r.line,
                    guess: false,
                });
            }
        }
        let reference = r.reference.as_str();
        let search = |kind: ProjectType| async move {
            let hits = p
                .search(reference, &manifest.minecraft, &manifest.loader, kind, 0, 5)
                .await
                .unwrap_or_default();
            (kind, hits)
        };
        let order = match r.project_type {
            Some(kind) => vec![kind],
            None => ProjectType::ALL.to_vec(),
        };
        let wanted = bulkadd::normal(reference);
        let mut found = vec![search(order[0]).await];
        if !bulkadd::has_exact(&wanted, &found[0].1) {
            found.extend(
                futures::future::join_all(order[1..].iter().map(|&kind| search(kind))).await,
            );
        }
        match bulkadd::best(&wanted, &found) {
            Some((kind, hit, score)) => Ok(bulkadd::BulkCandidate {
                provider,
                project_type: kind,
                project_id: hit.project_id.clone(),
                slug: hit.slug.clone(),
                name: hit.title.clone(),
                icon_url: hit.icon_url.clone(),
                raw: r.raw.clone(),
                line: r.line,
                guess: score < bulkadd::EXACT,
            }),
            None => Err(failure(&r)),
        }
    });
    let mut out = bulkadd::BulkLookup::default();
    for result in futures::future::join_all(lookups).await {
        match result {
            Ok(c) if existing.contains(&c.project_id) => out.in_pack.push(c),
            Ok(c) => out.found.push(c),
            Err(f) => out.failed.push(f),
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn identify_dropped(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    files: Vec<String>,
) -> Result<dropped::DroppedScan, String> {
    dropped::identify(&mr, &cf, &PathBuf::from(path), &files)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn add_dropped(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    items: Vec<dropped::DroppedFile>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let label = match items.as_slice() {
        [one] => format!("Add {}", one.filename),
        many => format!("Add {} files", many.len()),
    };
    let snapshot = |dir: &Path| {
        let folders = crate::ptype::Folders::load(dir);
        let mut snapshot = Snapshot::pack(dir, label);
        for item in items.iter().filter(|i| i.matched.is_none()) {
            snapshot = snapshot.file(
                &instance::overrides_dir(dir)
                    .join(folders.install_dir(item.project_type))
                    .join(&item.filename),
            );
        }
        snapshot
    };
    dropped::remember_unchecked(&dir, &items).map_err(es)?;
    mutate(&mr, &cf, &dir, snapshot, |dir| dropped::add(dir, &items)).await
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct FoundLocal {
    pub name: String,
    pub provider: ProviderId,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Rechecked {
    pub found: Vec<FoundLocal>,
    pub resolved: Option<PackResolved>,
}

#[tauri::command]
pub async fn recheck_dropped(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
) -> Result<Rechecked, String> {
    let dir = PathBuf::from(&path);
    let check = dropped::recheck(&mr, &cf, &dir).await.map_err(es)?;
    let found: Vec<FoundLocal> = check
        .found
        .iter()
        .filter_map(|f| {
            f.matched.as_ref().map(|m| FoundLocal {
                name: m.name.clone(),
                provider: m.provider,
            })
        })
        .collect();
    if found.is_empty() {
        dropped::set_unchecked(&dir, check.pending).map_err(es)?;
        return Ok(Rechecked {
            found,
            resolved: None,
        });
    }
    let label = match found.as_slice() {
        [one] => format!("Add {}", one.name),
        many => format!("Add {} files", many.len()),
    };
    let snapshot = |dir: &Path| {
        let mut snapshot = Snapshot::pack(dir, label);
        for path in &check.paths {
            snapshot = snapshot.file(path);
        }
        snapshot
    };
    let resolved = mutate(&mr, &cf, &dir, snapshot, |dir| {
        dropped::add(dir, &check.found)?;
        for path in &check.paths {
            crate::util::delete_file(path)?;
        }
        Ok(())
    })
    .await?;
    dropped::set_unchecked(&dir, check.pending).map_err(es)?;
    Ok(Rechecked {
        found,
        resolved: Some(resolved),
    })
}

#[tauri::command]
pub async fn remove_content(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    keys: Vec<String>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let label = format!("Remove {}", content::names_label(&dir, &keys));
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| content::delete_content(dir, &keys),
    )
    .await
}

#[tauri::command]
pub async fn set_content_disabled(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    keys: Vec<String>,
    disabled: bool,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let verb = if disabled { "Disable" } else { "Enable" };
    let label = format!("{verb} {}", content::names_label(&dir, &keys));
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| {
            for key in &keys {
                content::set_disabled(dir, key, disabled)?;
            }
            Ok(())
        },
    )
    .await
}

#[tauri::command]
pub async fn set_content_pinned(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    keys: Vec<String>,
    pinned: bool,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let verb = if pinned { "Pin" } else { "Unpin" };
    let label = format!("{verb} {}", content::names_label(&dir, &keys));
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| {
            for key in &keys {
                content::set_pinned(dir, key, pinned)?;
            }
            Ok(())
        },
    )
    .await
}

#[tauri::command]
pub async fn set_content_bundled(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    keys: Vec<String>,
    bundled: bool,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let verb = if bundled { "Bundle" } else { "Stop bundling" };
    let label = format!("{verb} {}", content::names_label(&dir, &keys));
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| {
            for key in &keys {
                content::set_no_bundle(dir, key, !bundled)?;
            }
            Ok(())
        },
    )
    .await
}

#[tauri::command]
pub async fn set_source_version(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    key: String,
    provider: ProviderId,
    version: Option<String>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let label = format!("Change {} on {provider}", content::name_of(&dir, &key));
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| {
            content::set_source_version(dir, &key, provider, version)?;
            Ok(())
        },
    )
    .await
}

#[tauri::command]
pub async fn set_content_explicit(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    key: String,
    explicit: bool,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let name = content::name_of(&dir, &key);
    let label = if explicit {
        format!("Keep {name} in the pack")
    } else {
        format!("Make {name} a dependency")
    };
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| content::set_explicit(dir, &key, explicit).map(|_| ()),
    )
    .await
}

#[derive(Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct VersionPin {
    pub key: String,
    pub version: String,
}

#[tauri::command]
pub async fn set_content_versions(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    updates: Vec<VersionPin>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let updates: Vec<VersionPin> = updates
        .into_iter()
        .filter(|u| !u.version.trim().is_empty())
        .collect();
    if updates.is_empty() {
        return Err("Pick a version.".into());
    }
    let keys: Vec<String> = updates.iter().map(|u| u.key.clone()).collect();
    let label = format!("Update {}", content::names_label(&dir, &keys));
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| {
            for u in &updates {
                content::set_version(dir, &u.key, u.version.clone())?;
            }
            Ok(())
        },
    )
    .await
}

#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct AltSource {
    pub key: String,
    pub provider: ProviderId,
    pub id: String,
}

#[tauri::command]
pub async fn add_alt_sources(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    sources: Vec<AltSource>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let label = match sources.as_slice() {
        [one] => format!(
            "Add {} to {}",
            one.provider.label(),
            content::name_of(&dir, &one.key)
        ),
        many => format!("Add sources to {} mods", many.len()),
    };
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| {
            for source in &sources {
                content::add_alt(dir, &source.key, source.provider, &source.id)?;
            }
            Ok(())
        },
    )
    .await
}

#[tauri::command]
pub async fn set_preferred_source(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    keys: Vec<String>,
    provider: ProviderId,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let what = match keys.as_slice() {
        [key] => content::name_of(&dir, key),
        _ => format!("{} mods", keys.len()),
    };
    let label = format!("Use {} for {what}", provider.label());
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| {
            for key in &keys {
                content::set_preferred(dir, key, provider)?;
            }
            Ok(())
        },
    )
    .await
}

#[tauri::command]
pub async fn remove_alt_source(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    key: String,
    provider: ProviderId,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let label = format!(
        "Remove {} from {}",
        provider.label(),
        content::name_of(&dir, &key)
    );
    mutate(
        &mr,
        &cf,
        &dir,
        |dir| Snapshot::pack(dir, label),
        |dir| content::remove_alt(dir, &key, provider).map(|_| ()),
    )
    .await
}

#[tauri::command]
pub async fn resolve_pack(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    force: bool,
) -> Result<PackResolved, String> {
    if force {
        mr.clear_version_cache()
            .map_err(|e| format!("Couldn't clear cached versions: {e}"))?;
    }
    do_resolve(&mr, &cf, &PathBuf::from(path)).await.map_err(es)
}

#[derive(Deserialize, Default, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "types.ts")]
pub struct ChangePreview {
    #[ts(as = "Option<Vec<VersionPin>>", optional)]
    pub versions: Vec<VersionPin>,
    #[ts(as = "Option<Vec<String>>", optional)]
    pub remove: Vec<String>,
    #[ts(as = "Option<Vec<String>>", optional)]
    pub disable: Vec<String>,
    #[ts(as = "Option<Vec<AddItem>>", optional)]
    pub add: Vec<AddItem>,
}

#[tauri::command]
pub async fn preview_change(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    change: ChangePreview,
) -> Result<resolver::ImpactReport, String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    let all = content::read_all(&dir);
    let known = super::known_items(&all);
    let aliases = resolver::aliases(&all);
    let excluded: HashSet<String> = all
        .iter()
        .filter(|i| i.disabled)
        .map(|i| i.project_id())
        .collect();
    let authored: Vec<ContentItem> = all
        .iter()
        .filter(|i| i.explicit && !i.disabled)
        .cloned()
        .collect();
    let base = resolver::resolve(&mr, &cf, &manifest, &authored, &excluded, &known, &aliases)
        .await
        .map_err(es)?;

    let mut next_authored = authored.clone();
    next_authored.retain(|i| {
        !change.remove.iter().any(|k| content::matches(i, k))
            && !change.disable.iter().any(|k| content::matches(i, k))
    });
    for pin in &change.versions {
        if pin.version.is_empty() {
            continue;
        }
        if let Some(item) = next_authored
            .iter_mut()
            .find(|i| content::matches(i, &pin.key))
        {
            let preferred = item.preferred;
            if let Some(src) = item.sources.get_mut(&preferred) {
                src.version_id = pin.version.clone();
            }
        }
    }
    next_authored.extend(change.add.iter().map(new_item));
    let mut next_excluded = excluded.clone();
    for key in change.remove.iter().chain(&change.disable) {
        if let Some(item) = all.iter().find(|i| content::matches(i, key)) {
            next_excluded.insert(item.project_id());
        }
    }
    let mut next_known = known.clone();
    for pin in &change.versions {
        if let Some(item) = all.iter().find(|i| content::matches(i, &pin.key)) {
            next_known.remove(&item.project_id());
        }
    }
    let next = resolver::resolve(
        &mr,
        &cf,
        &manifest,
        &next_authored,
        &next_excluded,
        &next_known,
        &aliases,
    )
    .await
    .map_err(es)?;

    let before: HashSet<String> = resolver::impact_problems(&base).into_iter().collect();
    let after: HashSet<String> = resolver::impact_problems(&next).into_iter().collect();
    let mut problems: Vec<String> = after.difference(&before).cloned().collect();
    let mut fixed: Vec<String> = before.difference(&after).cloned().collect();
    problems.sort();
    fixed.sort();
    let present: HashSet<String> = all.iter().map(|i| i.project_id()).collect();
    let before: HashSet<String> = resolver::suggestions(&base, &present)
        .into_iter()
        .map(|s| s.project_id)
        .collect();
    let added: HashSet<String> = change.add.iter().map(add_key).collect();
    let optional = resolver::suggestions(&next, &present)
        .into_iter()
        .filter(|s| !before.contains(&s.project_id) && !added.contains(&s.project_id))
        .collect();
    Ok(resolver::ImpactReport {
        changes: resolver::diff_locked(&base.locked, &next.locked),
        problems,
        fixed,
        optional,
    })
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct AvailableUpdate {
    pub key: String,
    pub project_id: String,
    pub version_id: String,
    pub version_number: String,
    pub version_type: String,
}

#[tauri::command]
pub async fn check_updates(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
) -> Result<Vec<AvailableUpdate>, String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    let lock = lockfile::read(&dir).map_err(es)?;
    let channel = manifest.channel.rank();
    let mr = mr.inner();
    let cf = cf.inner();
    let checks = lock
        .mods
        .iter()
        .filter(|m| !m.disabled && !m.pinned)
        .filter(|m| matches!(m.preferred, ProviderId::Modrinth | ProviderId::Curseforge))
        .map(|m| {
            let manifest = &manifest;
            async move {
                let provider = providers::by_id(m.preferred, mr, cf)?;
                let loader =
                    (m.project_type == ProjectType::Mod).then_some(manifest.loader.as_str());
                let versions = provider
                    .versions(&m.project_id, &manifest.minecraft, loader)
                    .await
                    .ok()?;
                let current = m.version_id();
                let best = versions
                    .iter()
                    .filter(|v| providers::channel_rank(&v.version_type) <= channel)
                    .max_by(|a, b| a.date_published.cmp(&b.date_published))?;
                if best.id == current {
                    return None;
                }
                let newer = versions
                    .iter()
                    .find(|v| v.id == current)
                    .map(|v| best.date_published > v.date_published)
                    .unwrap_or(true);
                newer.then(|| AvailableUpdate {
                    key: m.key.clone(),
                    project_id: m.project_id.clone(),
                    version_id: best.id.clone(),
                    version_number: best.version_number.clone(),
                    version_type: best.version_type.clone(),
                })
            }
        });
    Ok(futures::future::join_all(checks)
        .await
        .into_iter()
        .flatten()
        .collect())
}

#[tauri::command]
pub async fn list_unpublished(path: String) -> Result<Vec<instance::LocalContent>, String> {
    Ok(instance::list_unpublished(Path::new(&path)))
}
