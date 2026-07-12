use crate::ptype::ProjectType;
use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::collections::HashMap;

use crate::modrinth::{ModMeta, VersionInfo};

const API_BASE: &str = "https://api.curseforge.com";
const GAME_ID: u32 = 432;
pub const API_KEY: &str = match option_env!("CURSEFORGE_API_KEY") {
    Some(key) => key,
    None => "",
};

pub struct CurseForge {
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct ListResp<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct OneResp<T> {
    data: T,
}

#[derive(Deserialize)]
struct FingerprintResp {
    data: FingerprintData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintData {
    #[serde(default)]
    exact_matches: Vec<FingerprintMatch>,
}

#[derive(Deserialize)]
struct FingerprintMatch {
    id: u32,
    file: CfFile,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfMod {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub logo: Option<CfLogo>,
    #[serde(default)]
    pub authors: Vec<CfAuthor>,
    #[serde(default)]
    pub download_count: f64,
    #[serde(default)]
    pub class_id: Option<u32>,
}

pub fn type_from_class(id: Option<u32>) -> ProjectType {
    match id {
        Some(12) => ProjectType::Resourcepack,
        Some(6552) => ProjectType::Shader,
        _ => ProjectType::Mod,
    }
}

#[derive(Deserialize, Clone)]
pub struct CfLogo {
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize, Clone)]
pub struct CfAuthor {
    #[serde(default)]
    pub name: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfFile {
    pub id: u32,
    #[serde(default)]
    pub display_name: String,
    pub file_name: String,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub file_length: u64,
    #[serde(default)]
    pub hashes: Vec<CfHash>,
    #[serde(default)]
    pub release_type: u32,
    #[serde(default)]
    pub file_date: String,
    #[serde(default)]
    pub file_fingerprint: u64,
    #[serde(default)]
    pub dependencies: Vec<CfDep>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfDep {
    pub mod_id: u32,
    #[serde(default)]
    pub relation_type: u32,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfHash {
    pub value: String,
    pub algo: u32,
}

pub struct CfResolved {
    pub file_id: u32,
    pub version: String,
    pub filename: String,
    pub download_url: String,
    pub sha1: Option<String>,
    pub file_size: u64,
    pub name: String,
    pub slug: String,
    pub deps: Vec<CfDep>,
}

impl CurseForge {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self { client })
    }

    pub fn configured() -> bool {
        !API_KEY.is_empty()
    }

    fn ensure(&self) -> Result<()> {
        if API_KEY.is_empty() {
            bail!("CurseForge isn't set up yet.");
        }
        Ok(())
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> Result<T> {
        self.ensure()?;
        let resp = self
            .client
            .get(url)
            .header("x-api-key", API_KEY)
            .header("Accept", "application/json")
            .send()
            .await?;
        if !resp.status().is_success() {
            bail!("CurseForge request failed ({})", resp.status());
        }
        Ok(resp.json::<T>().await?)
    }

    pub async fn search(
        &self,
        query: &str,
        mc: &str,
        loader: &str,
        project_type: ProjectType,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<CfMod>> {
        let class_id = class_id(project_type);
        let loader_t = loader_type(loader);
        let page = limit.clamp(1, 50);
        let mut url = format!(
            "{API_BASE}/v1/mods/search?gameId={GAME_ID}&classId={class_id}&searchFilter={}&index={offset}&pageSize={page}&sortField=2&sortOrder=desc",
            urlencode(query)
        );
        if !mc.is_empty() {
            url.push_str(&format!("&gameVersion={mc}"));
        }
        if loader_t > 0 && project_type == ProjectType::Mod {
            url.push_str(&format!("&modLoaderType={loader_t}"));
        }
        let resp: ListResp<CfMod> = self.get_json(&url).await?;
        Ok(resp.data)
    }

    pub async fn get_mod(&self, id: u32) -> Result<CfMod> {
        let url = format!("{API_BASE}/v1/mods/{id}");
        let resp: OneResp<CfMod> = self.get_json(&url).await?;
        Ok(resp.data)
    }

    pub async fn mod_files(
        &self,
        id: u32,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<CfFile>> {
        let loader_t = loader_type(loader);
        let mut url = format!("{API_BASE}/v1/mods/{id}/files?pageSize=50");
        if !mc.is_empty() {
            url.push_str(&format!("&gameVersion={mc}"));
        }
        if loader_t > 0 {
            url.push_str(&format!("&modLoaderType={loader_t}"));
        }
        let resp: ListResp<CfFile> = self.get_json(&url).await?;
        Ok(resp.data)
    }

    pub async fn get_file(&self, mod_id: u32, file_id: u32) -> Result<CfFile> {
        let url = format!("{API_BASE}/v1/mods/{mod_id}/files/{file_id}");
        let resp: OneResp<CfFile> = self.get_json(&url).await?;
        Ok(resp.data)
    }

    pub fn download_url(&self, file: &CfFile) -> String {
        if let Some(u) = &file.download_url {
            if !u.is_empty() {
                return u.clone();
            }
        }
        let a = file.id / 1000;
        let b = file.id % 1000;
        format!(
            "https://edge.forgecdn.net/files/{a}/{b}/{}",
            file.file_name.replace(' ', "%20")
        )
    }

    pub async fn resolve(
        &self,
        mod_id: &str,
        pin: Option<&str>,
        mc: &str,
        loader: &str,
        channel: u8,
    ) -> Result<CfResolved> {
        let id: u32 = mod_id
            .parse()
            .map_err(|_| anyhow!("Invalid CurseForge id '{mod_id}'"))?;
        let file = match pin {
            Some(fid) if !fid.is_empty() => {
                let f: u32 = fid.parse().map_err(|_| {
                    anyhow!("Invalid CurseForge file id '{fid}'")
                })?;
                self.get_file(id, f).await?
            }
            _ => {
                let files = self.mod_files(id, mc, loader).await?;
                pick_file(files, channel).ok_or_else(|| {
                    anyhow!("No CurseForge file for Minecraft {mc} on {loader}")
                })?
            }
        };
        let info = self.get_mod(id).await.ok();
        Ok(CfResolved {
            file_id: file.id,
            version: file_version(&file),
            filename: file.file_name.clone(),
            download_url: self.download_url(&file),
            sha1: file
                .hashes
                .iter()
                .find(|h| h.algo == 1)
                .map(|h| h.value.clone()),
            file_size: file.file_length,
            name: info
                .as_ref()
                .map(|m| m.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| file.display_name.clone()),
            slug: info.map(|m| m.slug).unwrap_or_default(),
            deps: file.dependencies.clone(),
        })
    }

    pub async fn enrich(&self, ids: &[String]) -> Result<Vec<ModMeta>> {
        if !Self::configured() {
            return Ok(Vec::new());
        }
        let futs = ids.iter().map(|id| async move {
            let nid: u32 = id.parse().ok()?;
            let m = self.get_mod(nid).await.ok()?;
            Some(ModMeta {
                project_id: m.id.to_string(),
                icon_url: m.logo.map(|l| l.url).filter(|u| !u.is_empty()),
                author: m
                    .authors
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default(),
                author_icon_url: None,
            })
        });
        let results = futures::future::join_all(futs).await;
        Ok(results.into_iter().flatten().collect())
    }

    pub async fn versions(
        &self,
        id: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<VersionInfo>> {
        let nid: u32 = id
            .parse()
            .map_err(|_| anyhow!("Invalid CurseForge id '{id}'"))?;
        let mut files = self.mod_files(nid, mc, loader).await?;
        files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
        Ok(files
            .into_iter()
            .map(|f| VersionInfo {
                id: f.id.to_string(),
                version_number: file_version(&f),
                version_type: release_label(f.release_type).into(),
                date_published: f.file_date,
                game_versions: Vec::new(),
            })
            .collect())
    }

    pub async fn mods_named(
        &self,
        ids: &[String],
    ) -> HashMap<String, (String, String)> {
        if !Self::configured() {
            return HashMap::new();
        }
        let futs = ids.iter().map(|id| async move {
            let nid: u32 = id.parse().ok()?;
            let m = self.get_mod(nid).await.ok()?;
            Some((m.id.to_string(), (m.name, m.slug)))
        });
        futures::future::join_all(futs)
            .await
            .into_iter()
            .flatten()
            .collect()
    }

    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<CfMod>> {
        let url = format!(
            "{API_BASE}/v1/mods/search?gameId={GAME_ID}&slug={}&pageSize=10",
            urlencode(slug)
        );
        let resp: ListResp<CfMod> = self.get_json(&url).await?;
        Ok(resp
            .data
            .into_iter()
            .find(|m| m.slug.eq_ignore_ascii_case(slug)))
    }

    pub async fn fingerprints(
        &self,
        hashes: &[u32],
    ) -> Result<HashMap<u64, (u32, CfFile)>> {
        if hashes.is_empty() || !Self::configured() {
            return Ok(HashMap::new());
        }
        let url = format!("{API_BASE}/v1/fingerprints/{GAME_ID}");
        let body = serde_json::json!({ "fingerprints": hashes });
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", API_KEY)
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            bail!("CurseForge fingerprint request failed ({})", resp.status());
        }
        let parsed: FingerprintResp = resp.json().await?;
        let mut out: HashMap<u64, (u32, CfFile)> = HashMap::new();
        for m in parsed.data.exact_matches {
            out.insert(m.file.file_fingerprint, (m.id, m.file));
        }
        Ok(out)
    }
}

pub fn cf_fingerprint(data: &[u8]) -> u32 {
    let filtered: Vec<u8> = data
        .iter()
        .copied()
        .filter(|&b| b != 9 && b != 10 && b != 13 && b != 32)
        .collect();
    murmurhash2(&filtered, 1)
}

fn murmurhash2(data: &[u8], seed: u32) -> u32 {
    const M: u32 = 0x5bd1_e995;
    const R: u32 = 24;
    let len = data.len() as u32;
    let mut h: u32 = seed ^ len;
    let mut chunks = data.chunks_exact(4);
    for c in chunks.by_ref() {
        let mut k = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M);
        h ^= k;
    }
    let rem = chunks.remainder();
    if !rem.is_empty() {
        let mut tail: u32 = 0;
        for (i, &b) in rem.iter().enumerate() {
            tail |= (b as u32) << (8 * i as u32);
        }
        h ^= tail;
        h = h.wrapping_mul(M);
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;
    h
}

pub fn file_version(f: &CfFile) -> String {
    if f.display_name.is_empty() {
        f.file_name.clone()
    } else {
        f.display_name.clone()
    }
}

fn release_label(t: u32) -> &'static str {
    match t {
        2 => "beta",
        3 => "alpha",
        _ => "release",
    }
}

fn pick_file(mut files: Vec<CfFile>, channel: u8) -> Option<CfFile> {
    if files.is_empty() {
        return None;
    }
    files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
    let rank = |rt: u32| rt.saturating_sub(1).min(2) as u8;
    if let Some(f) = files.iter().find(|f| rank(f.release_type) <= channel) {
        return Some(f.clone());
    }
    files.into_iter().next()
}

fn class_id(project_type: ProjectType) -> u32 {
    match project_type {
        ProjectType::Resourcepack => 12,
        ProjectType::Shader => 6552,
        ProjectType::Mod => 6,
    }
}

fn loader_type(loader: &str) -> u32 {
    match loader {
        "forge" => 1,
        "fabric" => 4,
        "quilt" => 5,
        "neoforge" => 6,
        _ => 0,
    }
}

fn urlencode(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('&', "%26")
        .replace('#', "%23")
}
