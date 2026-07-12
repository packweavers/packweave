use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;

use crate::content;
use crate::curseforge::CurseForge;
use crate::manifest::{self, Manifest};
use crate::modrinth::{ModMeta, Modrinth, SearchHit, VersionInfo};
use crate::providers;
use crate::providers::ProviderId;
use crate::ptype::ProjectType;

use super::es;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub display_name: String,
    pub configured: bool,
}

#[tauri::command]
pub fn list_providers(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
) -> Vec<ProviderInfo> {
    providers::registry(state.inner(), cf.inner())
        .into_iter()
        .map(|p| ProviderInfo {
            id: p.id().to_string(),
            display_name: p.display_name().into(),
            configured: p.configured(),
        })
        .collect()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn search(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    provider: ProviderId,
    query: String,
    minecraft: String,
    loader: String,
    project_type: ProjectType,
    offset: u32,
    limit: u32,
) -> Result<Vec<SearchHit>, String> {
    let p = providers::by_id(provider, state.inner(), cf.inner())
        .ok_or_else(|| format!("Unknown provider '{provider}'."))?;
    if !p.configured() {
        return Err(format!("{} isn’t set up yet.", p.display_name()));
    }
    p.search(&query, &minecraft, &loader, project_type, offset, limit)
        .await
        .map_err(es)
}

#[derive(Clone)]
struct Cand {
    id: String,
    slug: String,
    name: String,
    icon_url: Option<String>,
    author: String,
}

fn pick_convert(pool: &[Cand], slug: &str, name: &str) -> Option<Cand> {
    let sl = slug.to_lowercase();
    let nl = name.to_lowercase();
    pool.iter()
        .find(|h| h.slug.to_lowercase() == sl)
        .or_else(|| pool.iter().find(|h| h.name.to_lowercase() == nl))
        .cloned()
}

async fn latest_version(
    p: &dyn providers::Provider,
    id: &str,
    project_type: ProjectType,
    manifest: &Manifest,
) -> String {
    let loader = if project_type == ProjectType::Mod {
        Some(manifest.loader.as_str())
    } else {
        None
    };
    p.versions(id, &manifest.minecraft, loader)
        .await
        .ok()
        .and_then(|v| v.into_iter().next())
        .map(|v| v.version_number)
        .unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
async fn finish_candidate(
    p: &dyn providers::Provider,
    target: ProviderId,
    id: String,
    slug: String,
    name: String,
    icon_url: Option<String>,
    fallback_author: String,
    project_type: ProjectType,
    manifest: &Manifest,
) -> ConvertCandidate {
    let meta = p
        .enrich(std::slice::from_ref(&id))
        .await
        .ok()
        .and_then(|v| v.into_iter().next());
    let author = meta
        .as_ref()
        .map(|m| m.author.clone())
        .filter(|a| !a.is_empty())
        .unwrap_or(fallback_author);
    let author_icon_url = meta.as_ref().and_then(|m| m.author_icon_url.clone());
    let icon_url =
        icon_url.or_else(|| meta.as_ref().and_then(|m| m.icon_url.clone()));
    let version = latest_version(p, &id, project_type, manifest).await;
    ConvertCandidate {
        id,
        slug,
        name,
        provider: target,
        icon_url,
        author,
        author_icon_url,
        version,
    }
}

fn parse_ref(raw: &str) -> String {
    let s = raw.trim();
    let s = s
        .split(['?', '#'])
        .next()
        .unwrap_or(s)
        .trim_end_matches('/');
    s.rsplit('/').next().unwrap_or(s).trim().to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertCandidate {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub provider: ProviderId,
    pub icon_url: Option<String>,
    pub author: String,
    pub author_icon_url: Option<String>,
    pub version: String,
}

#[tauri::command]
pub async fn convert_search(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
    target: ProviderId,
) -> Result<ConvertCandidate, String> {
    let dir = PathBuf::from(&path);
    let item = content::read_all(&dir)
        .into_iter()
        .find(|i| content::matches(i, &project_id))
        .ok_or_else(|| "Mod not found.".to_string())?;
    let manifest = manifest::read(&dir).map_err(es)?;
    let item_slug = item.active().map(|s| s.slug.clone()).unwrap_or_default();
    let query = if !item.name.is_empty() {
        item.name.clone()
    } else {
        item_slug.clone()
    };

    let provider = providers::by_id(target, state.inner(), cf.inner())
        .ok_or_else(|| format!("Unknown provider “{target}”."))?;
    let pool: Vec<Cand> = provider
        .search(
            &query,
            &manifest.minecraft,
            &manifest.loader,
            item.project_type,
            0,
            30,
        )
        .await
        .map_err(es)?
        .into_iter()
        .map(|h| Cand {
            id: h.project_id,
            slug: h.slug,
            name: h.title,
            icon_url: h.icon_url,
            author: h.author,
        })
        .collect();

    let cand =
        pick_convert(&pool, &item_slug, &item.name).ok_or_else(|| {
            format!(
                "Couldn’t find a matching {} project for “{}”.",
                target.label(),
                item.name
            )
        })?;

    Ok(finish_candidate(
        provider,
        target,
        cand.id,
        cand.slug,
        cand.name,
        cand.icon_url,
        cand.author,
        item.project_type,
        &manifest,
    )
    .await)
}

#[tauri::command]
pub async fn convert_lookup(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
    target: ProviderId,
    query: String,
) -> Result<ConvertCandidate, String> {
    let dir = PathBuf::from(&path);
    let item = content::read_all(&dir)
        .into_iter()
        .find(|i| content::matches(i, &project_id))
        .ok_or_else(|| "Mod not found.".to_string())?;
    let manifest = manifest::read(&dir).map_err(es)?;
    let r = parse_ref(&query);
    if r.is_empty() {
        return Err("Enter a link, slug, or id.".into());
    }

    let provider = providers::by_id(target, state.inner(), cf.inner())
        .ok_or_else(|| format!("Unknown provider “{target}”."))?;
    let proj = provider.lookup(&r).await.map_err(es)?;

    Ok(finish_candidate(
        provider,
        target,
        proj.id,
        proj.slug,
        proj.name,
        proj.icon_url,
        proj.author,
        item.project_type,
        &manifest,
    )
    .await)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichReq {
    pub id: String,
    pub provider: ProviderId,
}

#[tauri::command]
pub async fn enrich_mods(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    items: Vec<EnrichReq>,
) -> Result<Vec<ModMeta>, String> {
    let mut by_provider: HashMap<ProviderId, Vec<String>> = HashMap::new();
    for i in items {
        by_provider.entry(i.provider).or_default().push(i.id);
    }
    let mut out = Vec::new();
    for (prov, ids) in by_provider {
        if let Some(p) = providers::by_id(prov, state.inner(), cf.inner()) {
            if let Ok(metas) = p.enrich(&ids).await {
                out.extend(metas);
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn mod_versions(
    state: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    project_id: String,
    provider: ProviderId,
    minecraft: String,
    loader: Option<String>,
) -> Result<Vec<VersionInfo>, String> {
    let p = providers::by_id(provider, state.inner(), cf.inner())
        .ok_or_else(|| format!("Unknown provider '{provider}'."))?;
    p.versions(&project_id, &minecraft, loader.as_deref())
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn get_loader_versions(
    state: State<'_, Modrinth>,
    loader: String,
    minecraft: String,
) -> Result<Vec<String>, String> {
    state.loader_versions(&loader, &minecraft).await.map_err(es)
}

#[tauri::command]
pub async fn get_minecraft_versions(
    state: State<'_, Modrinth>,
) -> Result<Vec<String>, String> {
    state.game_versions().await.map_err(es)
}
