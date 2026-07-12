use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
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

use super::{do_resolve, es, PackResolved};

#[tauri::command]
pub async fn add_mod(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
    slug: String,
    name: String,
    project_type: ProjectType,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let exists = content::read_all(&dir)
        .iter()
        .any(|i| i.explicit && content::matches(i, &project_id));
    if !exists {
        let mut sources = BTreeMap::new();
        sources.insert(
            ProviderId::Modrinth,
            SourceFile {
                id: Some(project_id),
                slug,
                ..Default::default()
            },
        );
        content::add_item(
            &dir,
            &ContentItem {
                name,
                project_type,
                side: "auto".into(),
                explicit: true,
                disabled: false,
                preferred: ProviderId::Modrinth,
                sources,
                dependents: Vec::new(),
                client_side: "required".into(),
                server_side: "required".into(),
            },
        )
        .map_err(es)?;
    }
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn add_content(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    provider: ProviderId,
    project_id: String,
    slug: String,
    name: String,
    project_type: ProjectType,
    pin: Option<String>,
    url: Option<String>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let key = if provider == ProviderId::Url {
        url.clone().unwrap_or_else(|| project_id.clone())
    } else {
        project_id.clone()
    };
    let exists = content::read_all(&dir)
        .iter()
        .any(|i| i.explicit && content::matches(i, &key));
    if !exists {
        let display = if !name.is_empty() {
            name
        } else if !slug.is_empty() {
            slug.clone()
        } else {
            key.clone()
        };
        let src = match provider {
            ProviderId::Url => SourceFile {
                url: url.clone(),
                slug,
                ..Default::default()
            },
            _ => SourceFile {
                id: Some(project_id.clone()),
                slug,
                pin,
                ..Default::default()
            },
        };
        let mut sources = BTreeMap::new();
        sources.insert(provider, src);
        content::add_item(
            &dir,
            &ContentItem {
                name: display,
                project_type,
                side: "auto".into(),
                explicit: true,
                disabled: false,
                preferred: provider,
                sources,
                dependents: Vec::new(),
                client_side: "required".into(),
                server_side: "required".into(),
            },
        )
        .map_err(es)?;
    }
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn bulk_lookup(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    text: String,
) -> Result<bulkadd::BulkLookup, String> {
    let dir = PathBuf::from(&path);
    let existing: std::collections::HashSet<String> = content::read_all(&dir)
        .iter()
        .filter(|i| i.explicit)
        .filter_map(|i| i.active().and_then(|s| s.id.clone()))
        .collect();
    let mr = state.inner();
    let cfp = cf.inner();
    let futs = bulkadd::parse_refs(&text).into_iter().map(|r| async move {
        let candidates: Vec<ProviderId> = match r.provider {
            Some(p) => vec![p],
            None => vec![ProviderId::Modrinth, ProviderId::Curseforge],
        };
        for prov in candidates {
            if let Some(p) = providers::by_id(prov, mr, cfp) {
                if let Ok(proj) = p.lookup(&r.reference).await {
                    let project_type =
                        r.project_type.unwrap_or(proj.project_type);
                    return Ok(bulkadd::BulkCandidate {
                        provider: prov,
                        project_id: proj.id,
                        slug: proj.slug,
                        name: proj.name,
                        project_type,
                        icon_url: proj.icon_url,
                        raw: r.raw,
                    });
                }
            }
        }
        Err(bulkadd::BulkFailure {
            raw: r.raw,
            reason: "Not found on Modrinth or CurseForge".into(),
        })
    });
    let mut out = bulkadd::BulkLookup::default();
    let mut seen: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for res in futures::future::join_all(futs).await {
        match res {
            Ok(c) => {
                if existing.contains(&c.project_id)
                    || !seen.insert(c.project_id.clone())
                {
                    continue;
                }
                out.found.push(c);
            }
            Err(f) => out.failed.push(f),
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkAddItem {
    pub provider: ProviderId,
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub project_type: ProjectType,
}

#[tauri::command]
pub async fn add_content_bulk(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    items: Vec<BulkAddItem>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    for item in &items {
        if content::read_all(&dir)
            .iter()
            .any(|i| i.explicit && content::matches(i, &item.project_id))
        {
            continue;
        }
        let display = if !item.name.is_empty() {
            item.name.clone()
        } else if !item.slug.is_empty() {
            item.slug.clone()
        } else {
            item.project_id.clone()
        };
        let mut sources = BTreeMap::new();
        sources.insert(
            item.provider,
            SourceFile {
                id: Some(item.project_id.clone()),
                slug: item.slug.clone(),
                ..Default::default()
            },
        );
        content::add_item(
            &dir,
            &ContentItem {
                name: display,
                project_type: item.project_type,
                side: "auto".into(),
                explicit: true,
                disabled: false,
                preferred: item.provider,
                sources,
                dependents: Vec::new(),
                client_side: "required".into(),
                server_side: "required".into(),
            },
        )
        .map_err(es)?;
    }
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn remove_mod(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    content::remove_item(&dir, &project_id).map_err(es)?;
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn resolve_pack(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    force: bool,
) -> Result<PackResolved, String> {
    if force {
        state.clear_version_cache();
    }
    do_resolve(state.inner(), cf.inner(), &PathBuf::from(path))
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn promote_mod(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    content::set_explicit(&dir, &project_id, true).map_err(es)?;
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn set_content_disabled(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    key: String,
    disabled: bool,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    if disabled {
        if let Ok(lock) = lockfile::read(&dir) {
            if lock.mods.iter().any(|m| {
                m.project_id == key && !m.disabled && !m.dependents.is_empty()
            }) {
                return Err("Another mod depends on this. Disable or remove that one first.".into());
            }
        }
    }
    content::set_disabled(&dir, &key, disabled).map_err(es)?;
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn add_alt_source(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
    provider: ProviderId,
    id: String,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    content::add_alt(&dir, &project_id, provider, &id).map_err(es)?;
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn set_preferred_source(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
    provider: ProviderId,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    content::set_preferred(&dir, &project_id, provider).map_err(es)?;
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn remove_alt_source(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
    provider: ProviderId,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    content::remove_alt(&dir, &project_id, provider).map_err(es)?;
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn set_mod_version(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
    version: String,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    let pin = if version == "latest" || version.is_empty() {
        None
    } else {
        Some(version)
    };
    content::set_pin(&dir, &project_id, pin).map_err(es)?;
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionPin {
    pub project_id: String,
    pub version: String,
}

#[tauri::command]
pub async fn set_mod_versions(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    updates: Vec<VersionPin>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    for u in &updates {
        let pin = if u.version == "latest" || u.version.is_empty() {
            None
        } else {
            Some(u.version.clone())
        };
        content::set_pin(&dir, &u.project_id, pin).map_err(es)?;
    }
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn set_content_disabled_bulk(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    keys: Vec<String>,
    disabled: bool,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    if disabled {
        if let Ok(lock) = lockfile::read(&dir) {
            let blocked = lock.mods.iter().any(|m| {
                keys.contains(&m.project_id)
                    && !m.disabled
                    && m.dependents.iter().any(|d| !keys.contains(d))
            });
            if blocked {
                return Err(
                    "Some selected mods are required by mods you're keeping."
                        .into(),
                );
            }
        }
    }
    for key in &keys {
        content::set_disabled(&dir, key, disabled).map_err(es)?;
    }
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn remove_mods_bulk(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    keys: Vec<String>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    for key in &keys {
        content::remove_item(&dir, key).map_err(es)?;
    }
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn update_impact(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    updates: Vec<VersionPin>,
) -> Result<resolver::ImpactReport, String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    let base_authored = content::authored(&dir);
    let base =
        resolver::resolve(state.inner(), cf.inner(), &manifest, &base_authored)
            .await
            .map_err(es)?;
    let base_problems: std::collections::HashSet<String> =
        resolver::impact_problems(&base).into_iter().collect();

    let mut authored = base_authored;
    for u in &updates {
        if u.version.is_empty() || u.version == "latest" {
            continue;
        }
        if let Some(item) = authored
            .iter_mut()
            .find(|i| content::matches(i, &u.project_id))
        {
            let pref = item.preferred;
            if let Some(src) = item.sources.get_mut(&pref) {
                src.pin = Some(u.version.clone());
            }
        }
    }
    let out =
        resolver::resolve(state.inner(), cf.inner(), &manifest, &authored)
            .await
            .map_err(es)?;
    let problems: Vec<String> = resolver::impact_problems(&out)
        .into_iter()
        .filter(|p| !base_problems.contains(p))
        .collect();
    Ok(resolver::ImpactReport {
        changes: resolver::diff_locked(&base.locked, &out.locked),
        problems,
    })
}

#[tauri::command]
pub async fn list_unpublished(
    path: String,
) -> Result<Vec<instance::LocalContent>, String> {
    Ok(instance::list_unpublished(&PathBuf::from(path)))
}

#[tauri::command]
pub async fn identify_dropped(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    files: Vec<String>,
) -> Result<Vec<dropped::DroppedFile>, String> {
    dropped::identify(state.inner(), cf.inner(), &PathBuf::from(path), &files)
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn add_dropped(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    items: Vec<dropped::DroppedFile>,
) -> Result<PackResolved, String> {
    let dir = PathBuf::from(&path);
    dropped::add(&dir, &items).map_err(es)?;
    do_resolve(state.inner(), cf.inner(), &dir)
        .await
        .map_err(es)
}
