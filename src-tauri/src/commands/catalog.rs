use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::State;

use crate::content;
use crate::curseforge::CurseForge;
use crate::lockfile;
use crate::manifest::{self, Manifest};
use crate::modrinth::{LoaderVersions, ModMeta, Modrinth, SearchHit, VersionInfo};
use crate::providers;
use crate::providers::ProviderId;
use crate::ptype::ProjectType;

use super::es;
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub display_name: String,
    pub configured: bool,
}

#[tauri::command]
pub fn list_providers(mr: State<'_, Modrinth>, cf: State<'_, CurseForge>) -> Vec<ProviderInfo> {
    providers::registry(&mr, &cf)
        .into_iter()
        .map(|p| ProviderInfo {
            id: p.id(),
            display_name: p.display_name().into(),
            configured: p.configured(),
        })
        .collect()
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct CatalogHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    pub project_type: String,
    pub client_side: Option<String>,
    pub server_side: Option<String>,
}

impl From<SearchHit> for CatalogHit {
    fn from(hit: SearchHit) -> Self {
        CatalogHit {
            project_id: hit.project_id,
            slug: hit.slug,
            title: hit.title,
            description: hit.description,
            author: hit.author,
            downloads: hit.downloads,
            icon_url: hit.icon_url,
            categories: if hit.display_categories.is_empty() {
                hit.categories
            } else {
                hit.display_categories
            },
            project_type: hit.project_type,
            client_side: hit.client_side,
            server_side: hit.server_side,
        }
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn search(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    provider: ProviderId,
    query: String,
    minecraft: String,
    loader: String,
    project_type: ProjectType,
    offset: u32,
    limit: u32,
) -> Result<Vec<CatalogHit>, String> {
    let p = providers::by_id(provider, &mr, &cf)
        .ok_or_else(|| format!("Unknown provider '{provider}'."))?;
    if !p.configured() {
        return Err(format!("{} isn't set up yet.", p.display_name()));
    }
    let hits = p
        .search(&query, &minecraft, &loader, project_type, offset, limit)
        .await
        .map_err(es)?;
    Ok(hits.into_iter().map(CatalogHit::from).collect())
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
    let normal = crate::bulkadd::normal;
    let wanted: Vec<String> = [normal(slug), normal(name)]
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect();
    pool.iter()
        .find(|h| wanted.first() == Some(&normal(&h.slug)))
        .or_else(|| {
            pool.iter()
                .find(|h| wanted.contains(&normal(&h.slug)) || wanted.contains(&normal(&h.name)))
        })
        .cloned()
}

fn local_file(dir: &Path, item: &content::ContentItem) -> Option<Vec<u8>> {
    let filename = item.active()?.filename.clone();
    let instance = crate::packlocal::read(dir).instance_dir?;
    if filename.is_empty() {
        return None;
    }
    let folders = crate::ptype::Folders::load(dir);
    std::fs::read(
        Path::new(&instance)
            .join(folders.install_dir(item.project_type))
            .join(filename),
    )
    .ok()
}

async fn match_by_file(
    mr: &Modrinth,
    cf: &CurseForge,
    dir: &Path,
    item: &content::ContentItem,
    target: ProviderId,
) -> Option<Cand> {
    let active = item.active()?;
    match target {
        ProviderId::Modrinth => {
            let sha1 = active.sha1.clone().filter(|s| !s.is_empty())?;
            let found = mr
                .versions_by_hashes(std::slice::from_ref(&sha1))
                .await
                .ok()?;
            let project = mr.project(&found.get(&sha1)?.project_id).await.ok()?;
            Some(Cand {
                id: project.id,
                slug: project.slug,
                name: project.title,
                icon_url: project.icon_url,
                author: String::new(),
            })
        }
        ProviderId::Curseforge => {
            let bytes = match local_file(dir, item) {
                Some(bytes) => bytes,
                None if !active.download_url.is_empty() => {
                    mr.download(&active.download_url).await.ok()?
                }
                None => return None,
            };
            let fingerprint = crate::curseforge::cf_fingerprint(&bytes);
            let found = cf.fingerprints(&[fingerprint]).await.ok()?;
            let (mod_id, _) = found.get(&(fingerprint as u64))?;
            let m = cf.get_mod(*mod_id).await.ok()?;
            Some(Cand {
                id: m.id.to_string(),
                slug: m.slug,
                name: m.name,
                icon_url: m.logo.map(|l| l.url).filter(|u| !u.is_empty()),
                author: m
                    .authors
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default(),
            })
        }
        _ => None,
    }
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
    let icon_url = icon_url.or_else(|| meta.as_ref().and_then(|m| m.icon_url.clone()));
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

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
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
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
    target: ProviderId,
) -> Result<Option<ConvertCandidate>, String> {
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

    let provider = providers::by_id(target, &mr, &cf)
        .ok_or_else(|| format!("Unknown provider “{target}”."))?;
    if let Some(cand) = match_by_file(&mr, &cf, &dir, &item, target).await {
        return Ok(Some(
            finish_candidate(
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
            .await,
        ));
    }
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

    let Some(cand) = pick_convert(&pool, &item_slug, &item.name) else {
        return Ok(None);
    };

    Ok(Some(
        finish_candidate(
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
        .await,
    ))
}

#[tauri::command]
pub async fn convert_lookup(
    mr: State<'_, Modrinth>,
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

    let provider = providers::by_id(target, &mr, &cf)
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
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    items: Vec<EnrichReq>,
) -> Result<Vec<ModMeta>, String> {
    let mut by_provider: HashMap<ProviderId, Vec<String>> = HashMap::new();
    for i in items {
        by_provider.entry(i.provider).or_default().push(i.id);
    }
    let mut out = Vec::new();
    for (prov, ids) in by_provider {
        if let Some(p) = providers::by_id(prov, &mr, &cf) {
            if let Ok(metas) = p.enrich(&ids).await {
                out.extend(metas);
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn mod_versions(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    project_id: String,
    provider: ProviderId,
    minecraft: String,
    loader: Option<String>,
) -> Result<Vec<VersionInfo>, String> {
    let p = providers::by_id(provider, &mr, &cf)
        .ok_or_else(|| format!("Unknown provider '{provider}'."))?;
    p.versions(&project_id, &minecraft, loader.as_deref())
        .await
        .map_err(es)
}

#[tauri::command]
pub async fn get_loader_versions(
    mr: State<'_, Modrinth>,
    loader: String,
    minecraft: String,
) -> Result<LoaderVersions, String> {
    mr.loader_versions(&loader, &minecraft).await.map_err(es)
}

#[tauri::command]
pub async fn get_minecraft_versions(
    mr: State<'_, Modrinth>,
    include_snapshots: bool,
) -> Result<Vec<String>, String> {
    mr.game_versions(include_snapshots).await.map_err(es)
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ProjectDetails {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    pub url: String,
    pub downloads: u64,
}

#[tauri::command]
pub async fn project_details(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    provider: ProviderId,
    project_id: String,
) -> Result<ProjectDetails, String> {
    match provider {
        ProviderId::Curseforge => {
            let id: u32 = project_id
                .parse()
                .map_err(|_| "Not a CurseForge project id.".to_string())?;
            let m = cf.get_mod(id).await.map_err(es)?;
            let section = match crate::curseforge::type_from_class(m.class_id) {
                ProjectType::Resourcepack => "texture-packs",
                ProjectType::Shader => "shaders",
                ProjectType::Datapack => "data-packs",
                ProjectType::Mod => "mc-mods",
            };
            Ok(ProjectDetails {
                id: m.id.to_string(),
                url: format!("https://www.curseforge.com/minecraft/{section}/{}", m.slug),
                slug: m.slug,
                title: m.name,
                description: m.summary,
                icon_url: m.logo.map(|l| l.url).filter(|u| !u.is_empty()),
                client_side: None,
                server_side: None,
                downloads: m.download_count as u64,
            })
        }
        _ => {
            let p = mr.project(&project_id).await.map_err(es)?;
            Ok(ProjectDetails {
                url: format!("https://modrinth.com/{}/{}", p.project_type, p.slug),
                id: p.id,
                slug: p.slug,
                title: p.title,
                description: p.description,
                icon_url: p.icon_url,
                client_side: Some(p.client_side),
                server_side: Some(p.server_side),
                downloads: 0,
            })
        }
    }
}

#[tauri::command]
pub async fn version_changelog(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    provider: ProviderId,
    project_id: String,
    version_id: String,
) -> Result<String, String> {
    match provider {
        ProviderId::Curseforge => {
            let (Ok(mod_id), Ok(file_id)) = (project_id.parse::<u32>(), version_id.parse::<u32>())
            else {
                return Ok(String::new());
            };
            cf.file_changelog(mod_id, file_id).await.map_err(es)
        }
        _ => mr.version_changelog(&version_id).await.map_err(es),
    }
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct VersionNotes {
    pub version_id: String,
    pub version_number: String,
    pub version_type: String,
    pub date_published: String,
    pub changelog: String,
    pub html: bool,
}

const MAX_NOTES: usize = 25;

#[tauri::command]
pub async fn update_changelogs(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
    project_id: String,
    to_version: String,
) -> Result<Vec<VersionNotes>, String> {
    let dir = PathBuf::from(&path);
    let manifest = manifest::read(&dir).map_err(es)?;
    let lock = lockfile::read(&dir).map_err(es)?;
    let item = lock
        .mods
        .iter()
        .find(|m| m.project_id == project_id)
        .ok_or_else(|| "That isn't in the pack.".to_string())?;
    let provider = providers::by_id(item.preferred, &mr, &cf)
        .ok_or_else(|| "Changelogs come from Modrinth and CurseForge only.".to_string())?;
    let loader = (item.project_type == ProjectType::Mod).then_some(manifest.loader.as_str());
    let versions = provider
        .versions(&project_id, &manifest.minecraft, loader)
        .await
        .map_err(es)?;
    let Some(target) = versions.iter().find(|v| v.id == to_version) else {
        return Ok(Vec::new());
    };
    let limit = manifest
        .channel
        .rank()
        .max(providers::channel_rank(&target.version_type));
    let since = versions
        .iter()
        .find(|v| v.id == item.version_id())
        .map(|v| v.date_published.as_str());
    let mut picked: Vec<&VersionInfo> = versions
        .iter()
        .filter(|v| {
            v.id == target.id
                || (providers::channel_rank(&v.version_type) <= limit
                    && v.date_published <= target.date_published
                    && since.is_some_and(|since| v.date_published.as_str() > since))
        })
        .collect();
    picked.sort_by(|a, b| b.date_published.cmp(&a.date_published));
    picked.truncate(MAX_NOTES);
    let notes: Vec<(String, bool)> = match item.preferred {
        ProviderId::Curseforge => {
            let Ok(mod_id) = project_id.parse::<u32>() else {
                return Ok(Vec::new());
            };
            let cf = cf.inner();
            futures::future::join_all(picked.iter().map(|v| async move {
                let text = match v.id.parse::<u32>() {
                    Ok(file_id) => cf.file_changelog(mod_id, file_id).await.unwrap_or_default(),
                    Err(_) => String::new(),
                };
                (text, true)
            }))
            .await
        }
        _ => {
            let raw = mr
                .project_versions(&project_id, &manifest.minecraft, loader)
                .await
                .unwrap_or_default();
            let mut texts: HashMap<String, String> = raw
                .into_iter()
                .map(|v| (v.id, v.changelog.unwrap_or_default()))
                .collect();
            let mut out = Vec::new();
            for v in &picked {
                let text = match texts.remove(&v.id) {
                    Some(text) => text,
                    None => mr.version_changelog(&v.id).await.unwrap_or_default(),
                };
                out.push((text, false));
            }
            out
        }
    };
    Ok(picked
        .into_iter()
        .zip(notes)
        .map(|(v, (changelog, html))| VersionNotes {
            version_id: v.id.clone(),
            version_number: v.version_number.clone(),
            version_type: v.version_type.clone(),
            date_published: v.date_published.clone(),
            changelog,
            html,
        })
        .collect())
}
