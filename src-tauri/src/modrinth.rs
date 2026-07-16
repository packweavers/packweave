use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::cache::Cache;

const API: &str = "https://api.modrinth.com";
const USER_AGENT: &str =
    "packweave/0.1.0 (modpack builder; https://github.com/packweave)";
const MAX_CONCURRENT: usize = 8;
const FETCH_RETRIES: usize = 4;

const TTL_LIST: u64 = 3_600;
const TTL_META: u64 = 86_400;
const TTL_TAGS: u64 = 43_200;
const TTL_SEARCH: u64 = 600;

#[derive(Clone)]
pub struct Modrinth {
    client: reqwest::Client,
    version_cache: Arc<Mutex<HashMap<String, Vec<Version>>>>,
    version_by_id: Arc<Mutex<HashMap<String, Version>>>,
    semaphore: Arc<Semaphore>,
    cache: Arc<Cache>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub follows: u64,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub display_categories: Vec<String>,
    pub project_type: String,
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub icon_url: Option<String>,
    pub project_type: String,
    #[serde(default = "required")]
    pub client_side: String,
    #[serde(default = "required")]
    pub server_side: String,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub team: String,
    #[serde(default)]
    pub organization: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Organization {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: String,
    icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TeamMember {
    team_id: String,
    user: TeamUser,
    #[serde(default)]
    role: String,
}

#[derive(Debug, Deserialize)]
struct TeamUser {
    username: String,
    avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMeta {
    pub project_id: String,
    pub icon_url: Option<String>,
    pub author: String,
    pub author_icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    pub version_number: String,
    pub version_type: String,
    pub date_published: String,
    pub game_versions: Vec<String>,
}

fn required() -> String {
    "required".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFileHashes {
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub hashes: VersionFileHashes,
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub version_type: String,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub date_published: String,
}

#[derive(Debug, Deserialize)]
struct GameVersionTag {
    version: String,
    version_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoaderManifest {
    game_versions: Vec<LoaderGameVersion>,
}

#[derive(Debug, Deserialize)]
struct LoaderGameVersion {
    id: String,
    loaders: Vec<LoaderEntry>,
}

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    id: String,
    #[serde(default)]
    stable: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersions {
    pub versions: Vec<String>,
    pub recommended: Option<String>,
    pub kind: String,
}

impl Modrinth {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            version_cache: Arc::new(Mutex::new(HashMap::new())),
            version_by_id: Arc::new(Mutex::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT)),
            cache: Arc::new(Cache::new()),
        })
    }

    pub fn set_cache_dir(&self, dir: PathBuf) {
        self.cache.set_dir(dir);
    }

    async fn execute(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response> {
        let _permit = self.semaphore.clone().acquire_owned().await.ok();
        let mut attempt = 0usize;
        loop {
            let attempt_builder = builder
                .try_clone()
                .ok_or_else(|| anyhow!("request cannot be retried"))?;
            let resp = attempt_builder.send().await?;
            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }
            let retryable = status.as_u16() == 429 || status.is_server_error();
            if retryable && attempt < FETCH_RETRIES {
                attempt += 1;
                let reset = resp
                    .headers()
                    .get("x-ratelimit-reset")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());
                let wait = reset.unwrap_or((attempt as u64) * 2).clamp(1, 60);
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            let text = resp.text().await.unwrap_or_default();
            let snippet: String = text.chars().take(300).collect();
            return Err(anyhow!("Modrinth API error {status}: {snippet}"));
        }
    }

    pub async fn search(
        &self,
        query: &str,
        game_version: &str,
        loader: &str,
        project_type: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<SearchHit>> {
        let facets = if project_type == "mod" {
            format!(
                "[[\"project_type:{project_type}\"],[\"categories:{loader}\"],[\"versions:{game_version}\"]]"
            )
        } else {
            format!("[[\"project_type:{project_type}\"],[\"versions:{game_version}\"]]")
        };
        let cache_key = format!(
            "{query}|{game_version}|{loader}|{project_type}|{offset}|{limit}"
        );
        if let Some(hits) = self
            .cache
            .get::<Vec<SearchHit>>("search", &cache_key, TTL_SEARCH)
        {
            return Ok(hits);
        }
        let url = format!("{API}/v2/search");
        let resp = self
            .execute(self.client.get(&url).query(&[
                ("query", query),
                ("facets", facets.as_str()),
                ("limit", limit.to_string().as_str()),
                ("offset", offset.to_string().as_str()),
                ("index", "relevance"),
            ]))
            .await?;
        let parsed: SearchResponse = resp.json().await?;
        self.cache.put("search", &cache_key, &parsed.hits);
        Ok(parsed.hits)
    }

    pub async fn project_versions(
        &self,
        id: &str,
        game_version: &str,
        loader: Option<&str>,
    ) -> Result<Vec<Version>> {
        let key = format!("{id}|{game_version}|{}", loader.unwrap_or(""));
        let cached = self.version_cache.lock().unwrap().get(&key).cloned();
        if let Some(versions) = cached {
            return Ok(versions);
        }
        if let Some(versions) =
            self.cache.get::<Vec<Version>>("projver", &key, TTL_LIST)
        {
            self.version_cache
                .lock()
                .unwrap()
                .insert(key, versions.clone());
            return Ok(versions);
        }
        let url = format!("{API}/v2/project/{id}/version");
        let mut params: Vec<(&str, String)> =
            vec![("game_versions", format!("[\"{game_version}\"]"))];
        if let Some(l) = loader {
            params.push(("loaders", format!("[\"{l}\"]")));
        }
        let resp = self.execute(self.client.get(&url).query(&params)).await?;
        let versions: Vec<Version> = resp.json().await?;
        self.cache.put("projver", &key, &versions);
        self.version_cache
            .lock()
            .unwrap()
            .insert(key, versions.clone());
        Ok(versions)
    }

    pub async fn version(&self, id: &str) -> Result<Version> {
        if let Some(v) = self.version_by_id.lock().unwrap().get(id).cloned() {
            return Ok(v);
        }
        if let Some(v) = self.cache.get::<Version>("version", id, 0) {
            self.version_by_id
                .lock()
                .unwrap()
                .insert(id.to_string(), v.clone());
            return Ok(v);
        }
        let url = format!("{API}/v2/version/{id}");
        let resp = self.execute(self.client.get(&url)).await?;
        let version: Version = resp.json().await?;
        self.cache.put("version", id, &version);
        self.version_by_id
            .lock()
            .unwrap()
            .insert(id.to_string(), version.clone());
        Ok(version)
    }

    pub async fn project(&self, id: &str) -> Result<Project> {
        if let Some(p) = self.cache.get::<Project>("project", id, TTL_META) {
            return Ok(p);
        }
        let url = format!("{API}/v2/project/{id}");
        let resp = self.execute(self.client.get(&url)).await?;
        let project: Project = resp.json().await?;
        self.cache.put("project", id, &project);
        Ok(project)
    }

    pub async fn versions_by_hashes(
        &self,
        hashes: &[String],
    ) -> Result<HashMap<String, Version>> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }
        let url = format!("{API}/v2/version_files");
        let body = serde_json::json!({ "hashes": hashes, "algorithm": "sha1" });
        let resp = self.execute(self.client.post(&url).json(&body)).await?;
        let map: HashMap<String, Version> = resp.json().await?;
        let mut cache = self.version_by_id.lock().unwrap();
        for v in map.values() {
            cache.entry(v.id.clone()).or_insert_with(|| v.clone());
        }
        Ok(map)
    }

    pub async fn projects_bulk(&self, ids: &[String]) -> Result<Vec<Project>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let ids_json = serde_json::to_string(ids)?;
        let url = format!("{API}/v2/projects");
        let resp = self
            .execute(self.client.get(&url).query(&[("ids", ids_json.as_str())]))
            .await?;
        Ok(resp.json().await?)
    }

    async fn teams_bulk(&self, ids: &[String]) -> Result<Vec<Vec<TeamMember>>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let ids_json = serde_json::to_string(ids)?;
        let url = format!("{API}/v2/teams");
        let resp = self
            .execute(self.client.get(&url).query(&[("ids", ids_json.as_str())]))
            .await?;
        Ok(resp.json().await?)
    }

    async fn orgs_bulk(&self, ids: &[String]) -> Result<Vec<Organization>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let ids_json = serde_json::to_string(ids)?;
        let url = format!("{API}/v3/organizations");
        let resp = self
            .execute(self.client.get(&url).query(&[("ids", ids_json.as_str())]))
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn enrich(&self, ids: &[String]) -> Result<Vec<ModMeta>> {
        let mut out: Vec<ModMeta> = Vec::new();
        let mut missing: Vec<String> = Vec::new();
        for id in ids {
            match self.cache.get::<ModMeta>("meta", id, TTL_META) {
                Some(m) => out.push(m),
                None => missing.push(id.clone()),
            }
        }
        if missing.is_empty() {
            return Ok(out);
        }

        let projects = self.projects_bulk(&missing).await?;
        let team_ids: Vec<String> = projects
            .iter()
            .map(|p| p.team.clone())
            .filter(|t| !t.is_empty())
            .collect();
        let teams = self.teams_bulk(&team_ids).await.unwrap_or_default();
        let mut owner: HashMap<String, (String, Option<String>)> =
            HashMap::new();
        for team in &teams {
            if team.is_empty() {
                continue;
            }
            let tid = team[0].team_id.clone();
            let member = team
                .iter()
                .find(|m| m.role.eq_ignore_ascii_case("owner"))
                .or_else(|| team.first());
            if let Some(m) = member {
                owner.insert(
                    tid,
                    (m.user.username.clone(), m.user.avatar_url.clone()),
                );
            }
        }
        let org_ids: Vec<String> = projects
            .iter()
            .filter_map(|p| p.organization.clone())
            .filter(|o| !o.is_empty())
            .collect();
        let org_map: HashMap<String, (String, Option<String>)> = self
            .orgs_bulk(&org_ids)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|o| {
                let name = if o.name.is_empty() { o.slug } else { o.name };
                (o.id, (name, o.icon_url))
            })
            .collect();

        for p in projects {
            let (author, author_icon_url) = p
                .organization
                .as_ref()
                .and_then(|o| org_map.get(o))
                .cloned()
                .or_else(|| owner.get(&p.team).cloned())
                .unwrap_or_default();
            let meta = ModMeta {
                project_id: p.id.clone(),
                icon_url: p.icon_url,
                author,
                author_icon_url,
            };
            self.cache.put("meta", &p.id, &meta);
            out.push(meta);
        }
        Ok(out)
    }

    pub async fn mod_versions(
        &self,
        id: &str,
        game_version: &str,
        loader: Option<&str>,
    ) -> Result<Vec<VersionInfo>> {
        let mut versions =
            self.project_versions(id, game_version, loader).await?;
        versions.sort_by(|a, b| b.date_published.cmp(&a.date_published));
        Ok(versions
            .into_iter()
            .map(|v| VersionInfo {
                id: v.id,
                version_number: v.version_number,
                version_type: v.version_type,
                date_published: v.date_published,
                game_versions: v.game_versions,
            })
            .collect())
    }

    pub async fn game_versions(
        &self,
        include_snapshots: bool,
    ) -> Result<Vec<String>> {
        let tags: Vec<(String, String)> = match self
            .cache
            .get::<Vec<(String, String)>>("gamever", "all", TTL_TAGS)
        {
            Some(v) => v,
            None => {
                let url = format!("{API}/v2/tag/game_version");
                let resp = self.execute(self.client.get(&url)).await?;
                let list: Vec<GameVersionTag> = resp.json().await?;
                let tags: Vec<(String, String)> = list
                    .into_iter()
                    .map(|g| (g.version, g.version_type))
                    .collect();
                self.cache.put("gamever", "all", &tags);
                tags
            }
        };
        Ok(tags
            .into_iter()
            .filter(|(_, ty)| include_snapshots || ty == "release")
            .map(|(v, _)| v)
            .collect())
    }

    pub async fn loader_versions(
        &self,
        loader: &str,
        minecraft: &str,
    ) -> Result<LoaderVersions> {
        let path = match loader {
            "fabric" => "fabric",
            "quilt" => "quilt",
            "forge" => "forge",
            "neoforge" => "neo",
            _ => return Ok(LoaderVersions::default()),
        };
        let cache_key = format!("{loader}|{minecraft}");
        let versions: Vec<(String, bool)> = match self
            .cache
            .get::<Vec<(String, bool)>>("loaders", &cache_key, TTL_TAGS)
        {
            Some(v) => v,
            None => {
                let url = format!(
                    "https://launcher-meta.modrinth.com/{path}/v0/manifest.json"
                );
                let resp = self.execute(self.client.get(&url)).await?;
                let manifest: LoaderManifest = resp.json().await?;
                let entry = manifest
                    .game_versions
                    .iter()
                    .find(|g| g.id == minecraft && !g.loaders.is_empty())
                    .or_else(|| {
                        manifest
                            .game_versions
                            .iter()
                            .find(|g| g.id == "${modrinth.gameVersion}")
                    });
                let versions: Vec<(String, bool)> = entry
                    .map(|e| {
                        e.loaders
                            .iter()
                            .map(|l| (l.id.clone(), l.stable))
                            .collect()
                    })
                    .unwrap_or_default();
                self.cache.put("loaders", &cache_key, &versions);
                versions
            }
        };
        let stable = versions.iter().find(|(_, s)| *s).map(|(id, _)| id);
        let (recommended, kind) = match stable {
            Some(id) => (Some(id.clone()), "stable"),
            None => (
                versions.first().map(|(id, _)| id.clone()),
                if versions.is_empty() { "" } else { "latest" },
            ),
        };
        Ok(LoaderVersions {
            versions: versions.into_iter().map(|(id, _)| id).collect(),
            recommended,
            kind: kind.into(),
        })
    }

    pub async fn versions_from_hashes(
        &self,
        sha1_hashes: &[String],
    ) -> Result<HashMap<String, Version>> {
        if sha1_hashes.is_empty() {
            return Ok(HashMap::new());
        }
        let url = format!("{API}/v2/version_files");
        let body =
            serde_json::json!({ "hashes": sha1_hashes, "algorithm": "sha1" });
        let resp = self.execute(self.client.post(&url).json(&body)).await?;
        Ok(resp.json().await?)
    }

    pub async fn download(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.execute(self.client.get(url)).await?;
        Ok(resp.bytes().await?.to_vec())
    }

    pub fn clear_version_cache(&self) {
        self.version_cache.lock().unwrap().clear();
        self.version_by_id.lock().unwrap().clear();
        self.cache.clear("projver");
        self.cache.clear("search");
    }
}
