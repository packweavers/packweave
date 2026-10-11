use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::curseforge::CurseForge;
use crate::modrinth::{ModMeta, Modrinth, SearchHit, Version, VersionFile, VersionInfo};
use crate::ptype::ProjectType;
use ts_rs::TS;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize, TS,
)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "types.ts")]
pub enum ProviderId {
    #[default]
    Modrinth,
    Curseforge,
    Url,
    Local,
}

impl ProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::Modrinth => "modrinth",
            ProviderId::Curseforge => "curseforge",
            ProviderId::Url => "url",
            ProviderId::Local => "local",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ProviderId::Modrinth => "Modrinth",
            ProviderId::Curseforge => "CurseForge",
            ProviderId::Url => "URL",
            ProviderId::Local => "Local file",
        }
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug)]
pub struct NoVersion {
    pub minecraft: String,
    pub loader: String,
}

impl fmt::Display for NoVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.loader.is_empty() {
            write!(f, "No version for Minecraft {}", self.minecraft)
        } else {
            write!(
                f,
                "No version for Minecraft {} on {}",
                self.minecraft,
                loader_name(&self.loader)
            )
        }
    }
}

impl std::error::Error for NoVersion {}

pub fn no_version(minecraft: &str, loader: &str) -> NoVersion {
    NoVersion {
        minecraft: minecraft.to_string(),
        loader: loader.to_string(),
    }
}

pub fn loader_name(loader: &str) -> String {
    match loader {
        "fabric" => "Fabric".into(),
        "forge" => "Forge".into(),
        "neoforge" => "NeoForge".into(),
        "quilt" => "Quilt".into(),
        "vanilla" => "Vanilla".into(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    Required,
    Optional,
    Incompatible,
}

pub struct ResolvedDep {
    pub provider: ProviderId,
    pub id: String,
    pub version: Option<String>,
    pub kind: DepKind,
}

pub struct Resolved {
    pub slug: String,
    pub name: String,
    pub version_id: String,
    pub version_number: String,
    pub filename: String,
    pub download_url: String,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub file_size: u64,
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    pub deps: Vec<ResolvedDep>,
    pub actual_type: Option<ProjectType>,
}

pub struct ProviderProject {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub author: String,
    pub project_type: ProjectType,
}

#[async_trait]
#[allow(clippy::double_must_use)]
pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn display_name(&self) -> &'static str;
    fn configured(&self) -> bool;

    async fn search(
        &self,
        query: &str,
        mc: &str,
        loader: &str,
        project_type: ProjectType,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<SearchHit>>;

    async fn enrich(&self, ids: &[String]) -> Result<Vec<ModMeta>>;

    async fn versions(&self, id: &str, mc: &str, loader: Option<&str>) -> Result<Vec<VersionInfo>>;

    async fn lookup(&self, reference: &str) -> Result<ProviderProject>;

    async fn names(&self, ids: &[String]) -> HashMap<String, (String, String)>;

    async fn resolve(
        &self,
        id: &str,
        version: Option<&str>,
        mc: &str,
        loader: &str,
        project_type: ProjectType,
        channel: u8,
    ) -> Result<Resolved>;
}

pub fn registry<'a>(mr: &'a Modrinth, cf: &'a CurseForge) -> Vec<&'a dyn Provider> {
    vec![mr as &dyn Provider, cf as &dyn Provider]
}

fn not_content(name: &str) -> anyhow::Error {
    anyhow!("“{name}” isn't a mod, resource pack, shader or data pack.")
}

pub fn by_id<'a>(id: ProviderId, mr: &'a Modrinth, cf: &'a CurseForge) -> Option<&'a dyn Provider> {
    registry(mr, cf).into_iter().find(|p| p.id() == id)
}

pub fn channel_rank(channel: &str) -> u8 {
    match channel {
        "alpha" => 2,
        "beta" => 1,
        _ => 0,
    }
}

pub fn pick_best(mut versions: Vec<Version>, channel: u8) -> Option<Version> {
    if versions.is_empty() {
        return None;
    }
    versions.sort_by(|a, b| b.date_published.cmp(&a.date_published));
    if let Some(v) = versions
        .iter()
        .find(|v| channel_rank(&v.version_type) <= channel)
    {
        return Some(v.clone());
    }
    versions.into_iter().next()
}

fn wanted_extension(project_type: ProjectType) -> Option<&'static str> {
    match project_type {
        ProjectType::Mod => Some(".jar"),
        ProjectType::Datapack => Some(".zip"),
        _ => None,
    }
}

pub fn pick_file(files: &[VersionFile], project_type: ProjectType) -> Option<VersionFile> {
    let wanted = wanted_extension(project_type);
    let fits = |f: &&VersionFile| wanted.is_none_or(|ext| f.filename.to_lowercase().ends_with(ext));
    files
        .iter()
        .filter(fits)
        .find(|f| f.primary)
        .or_else(|| files.iter().find(fits))
        .or_else(|| files.iter().find(|f| f.primary))
        .or_else(|| files.first())
        .cloned()
}

pub fn file_type(
    requested: ProjectType,
    filename: &str,
    loaders: &[String],
) -> Option<ProjectType> {
    let name = filename.to_lowercase();
    match requested {
        ProjectType::Datapack if name.ends_with(".jar") => Some(ProjectType::Mod),
        ProjectType::Mod if name.ends_with(".zip") && loaders.iter().any(|l| l == "datapack") => {
            Some(ProjectType::Datapack)
        }
        _ => None,
    }
}

#[async_trait]
impl Provider for Modrinth {
    fn id(&self) -> ProviderId {
        ProviderId::Modrinth
    }

    fn display_name(&self) -> &'static str {
        "Modrinth"
    }

    fn configured(&self) -> bool {
        true
    }

    async fn search(
        &self,
        query: &str,
        mc: &str,
        loader: &str,
        project_type: ProjectType,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<SearchHit>> {
        Modrinth::search(
            self,
            query,
            mc,
            loader,
            project_type.as_str(),
            offset,
            limit,
        )
        .await
    }

    async fn enrich(&self, ids: &[String]) -> Result<Vec<ModMeta>> {
        Modrinth::enrich(self, ids).await
    }

    async fn versions(&self, id: &str, mc: &str, loader: Option<&str>) -> Result<Vec<VersionInfo>> {
        Modrinth::mod_versions(self, id, mc, loader).await
    }

    async fn lookup(&self, reference: &str) -> Result<ProviderProject> {
        let p = Modrinth::project(self, reference).await?;
        let project_type = match p.project_type.as_str() {
            "modpack" => return Err(not_content(&p.title)),
            "mod" if p.is_datapack() => ProjectType::Datapack,
            other => ProjectType::from_api(other),
        };
        let author = Modrinth::enrich(self, std::slice::from_ref(&p.id))
            .await
            .ok()
            .and_then(|v| v.into_iter().next())
            .map(|m| m.author)
            .unwrap_or_default();
        Ok(ProviderProject {
            project_type,
            id: p.id,
            slug: p.slug,
            name: p.title,
            icon_url: p.icon_url,
            author,
        })
    }

    async fn names(&self, ids: &[String]) -> HashMap<String, (String, String)> {
        Modrinth::projects_bulk(self, ids)
            .await
            .map(|ps| ps.into_iter().map(|p| (p.id, (p.title, p.slug))).collect())
            .unwrap_or_default()
    }

    async fn resolve(
        &self,
        id: &str,
        version: Option<&str>,
        mc: &str,
        loader: &str,
        project_type: ProjectType,
        channel: u8,
    ) -> Result<Resolved> {
        let lf = project_type.version_loader(loader);
        let version = match version {
            Some(p) if !p.is_empty() => Modrinth::version(self, p).await?,
            _ => pick_best(Modrinth::project_versions(self, id, mc, lf).await?, channel)
                .ok_or_else(|| no_version(mc, loader))?,
        };
        let proj = Modrinth::project(self, id).await?;
        let (download_url, filename, sha1, sha512, file_size) =
            match pick_file(&version.files, project_type) {
                Some(f) => (f.url, f.filename, f.hashes.sha1, f.hashes.sha512, f.size),
                None => (String::new(), String::new(), None, None, 0),
            };
        let actual_type = file_type(project_type, &filename, &version.loaders);
        let mut deps = Vec::new();
        for d in &version.dependencies {
            let kind = match d.dependency_type.as_str() {
                "required" => DepKind::Required,
                "optional" => DepKind::Optional,
                "incompatible" => DepKind::Incompatible,
                _ => continue,
            };
            let pid = match d.project_id.clone() {
                Some(p) => Some(p),
                None => match &d.version_id {
                    Some(vid) => Modrinth::version(self, vid)
                        .await
                        .ok()
                        .map(|v| v.project_id),
                    None => None,
                },
            };
            if let Some(pid) = pid {
                deps.push(ResolvedDep {
                    provider: ProviderId::Modrinth,
                    id: pid,
                    version: d.version_id.clone(),
                    kind,
                });
            }
        }
        Ok(Resolved {
            slug: proj.slug,
            name: proj.title,
            version_id: version.id,
            version_number: version.version_number,
            filename,
            download_url,
            sha1,
            sha512,
            file_size,
            client_side: Some(proj.client_side),
            server_side: Some(proj.server_side),
            deps,
            actual_type,
        })
    }
}

#[async_trait]
impl Provider for CurseForge {
    fn id(&self) -> ProviderId {
        ProviderId::Curseforge
    }

    fn display_name(&self) -> &'static str {
        "CurseForge"
    }

    fn configured(&self) -> bool {
        CurseForge::configured()
    }

    async fn search(
        &self,
        query: &str,
        mc: &str,
        loader: &str,
        project_type: ProjectType,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<SearchHit>> {
        let mods = CurseForge::search(self, query, mc, loader, project_type, offset, limit).await?;
        Ok(mods
            .into_iter()
            .map(|m| SearchHit {
                project_id: m.id.to_string(),
                slug: m.slug,
                title: m.name,
                description: m.summary,
                author: m
                    .authors
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default(),
                downloads: m.download_count as u64,
                follows: 0,
                icon_url: m.logo.map(|l| l.url),
                categories: Vec::new(),
                display_categories: Vec::new(),
                project_type: project_type.as_str().to_string(),
                client_side: None,
                server_side: None,
                versions: Vec::new(),
            })
            .collect())
    }

    async fn enrich(&self, ids: &[String]) -> Result<Vec<ModMeta>> {
        CurseForge::enrich(self, ids).await
    }

    async fn versions(&self, id: &str, mc: &str, loader: Option<&str>) -> Result<Vec<VersionInfo>> {
        CurseForge::versions(self, id, mc, loader.unwrap_or("")).await
    }

    async fn lookup(&self, reference: &str) -> Result<ProviderProject> {
        let m = if reference.chars().all(|c| c.is_ascii_digit()) {
            let nid: u32 = reference
                .parse()
                .map_err(|_| anyhow!("Invalid CurseForge id."))?;
            CurseForge::get_mod(self, nid).await?
        } else {
            CurseForge::find_by_slug(self, reference)
                .await?
                .ok_or_else(|| anyhow!("No CurseForge project for slug “{reference}”."))?
        };
        let project_type =
            crate::curseforge::content_type(m.class_id).ok_or_else(|| not_content(&m.name))?;
        Ok(ProviderProject {
            project_type,
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

    async fn names(&self, ids: &[String]) -> HashMap<String, (String, String)> {
        CurseForge::mods_named(self, ids).await
    }

    async fn resolve(
        &self,
        id: &str,
        version: Option<&str>,
        mc: &str,
        loader: &str,
        project_type: ProjectType,
        channel: u8,
    ) -> Result<Resolved> {
        let cf_loader = if project_type == ProjectType::Mod {
            loader
        } else {
            ""
        };
        let r = CurseForge::resolve(self, id, version, mc, cf_loader, channel).await?;
        let deps = r
            .deps
            .iter()
            .filter_map(|d| {
                let kind = match d.relation_type {
                    3 => DepKind::Required,
                    2 => DepKind::Optional,
                    5 => DepKind::Incompatible,
                    _ => return None,
                };
                Some(ResolvedDep {
                    provider: ProviderId::Curseforge,
                    id: d.mod_id.to_string(),
                    version: None,
                    kind,
                })
            })
            .collect();
        let actual_type = file_type(project_type, &r.filename, &[]);
        Ok(Resolved {
            slug: if r.slug.is_empty() {
                id.to_string()
            } else {
                r.slug
            },
            name: if r.name.is_empty() {
                id.to_string()
            } else {
                r.name
            },
            version_id: r.file_id.to_string(),
            version_number: r.version,
            filename: r.filename,
            download_url: r.download_url,
            sha1: r.sha1,
            sha512: None,
            file_size: r.file_size,
            client_side: None,
            server_side: None,
            deps,
            actual_type,
        })
    }
}
