use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::content::{self, ContentItem};
use crate::curseforge::{self, CurseForge};
use crate::lockfile::SourceFile;
use crate::modmeta;
use crate::modrinth::Modrinth;
use crate::packlocal;
use crate::providers::ProviderId;
use crate::ptype::{Folders, ProjectType};
use crate::validate;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct DroppedMatch {
    pub provider: ProviderId,
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub version_id: String,
    pub version_number: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct DroppedFile {
    pub path: String,
    pub filename: String,
    pub project_type: ProjectType,
    pub already_in_pack: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub matched: Option<DroppedMatch>,
    #[serde(default)]
    pub unchecked: bool,
}

fn classify(path: &Path) -> ProjectType {
    if let Some(meta) = modmeta::read_archive(path) {
        if meta.loaders.iter().any(|l| l == "datapack") {
            return ProjectType::Datapack;
        }
        if meta.pack_format.is_some() {
            return ProjectType::Resourcepack;
        }
        if meta.loaders.iter().any(|l| l == "iris") {
            return ProjectType::Shader;
        }
        return ProjectType::Mod;
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
    {
        Some(ext) if ext == "zip" => ProjectType::Resourcepack,
        _ => ProjectType::Mod,
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct DroppedScan {
    pub files: Vec<DroppedFile>,
    pub failures: Vec<String>,
}

pub async fn identify(
    mr: &Modrinth,
    cf: &CurseForge,
    pack_dir: &Path,
    files: &[String],
) -> Result<DroppedScan> {
    let existing: std::collections::HashSet<String> = content::read_all(pack_dir)
        .iter()
        .filter_map(|i| i.active().and_then(|s| s.id.clone()))
        .collect();

    let mut out = Vec::new();
    let mut hashes = Vec::new();
    let mut prints = Vec::new();
    for f in files {
        let p = Path::new(f);
        if !p.is_file() {
            continue;
        }
        let filename = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| f.clone());
        let bytes = match std::fs::read(p) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let mut h = Sha1::new();
        h.update(&bytes);
        hashes.push(hex::encode(h.finalize()));
        prints.push(curseforge::cf_fingerprint(&bytes));
        out.push(DroppedFile {
            path: f.clone(),
            filename,
            project_type: classify(p),
            already_in_pack: false,
            matched: None,
            unchecked: false,
        });
    }

    let mut failures = Vec::new();
    let mr_found = match mr.versions_by_hashes(&hashes).await {
        Ok(found) => found,
        Err(e) => {
            failures.push(validate::reason(&e.to_string(), ProviderId::Modrinth).0);
            Default::default()
        }
    };
    let cf_found = match cf.fingerprints(&prints).await {
        Ok(found) => found,
        Err(e) => {
            failures.push(validate::reason(&e.to_string(), ProviderId::Curseforge).0);
            Default::default()
        }
    };

    for (i, file) in out.iter_mut().enumerate() {
        if let Some(v) = mr_found.get(&hashes[i]) {
            let proj = mr.project(&v.project_id).await.ok();
            file.matched = Some(DroppedMatch {
                provider: ProviderId::Modrinth,
                project_id: v.project_id.clone(),
                slug: proj.as_ref().map(|p| p.slug.clone()).unwrap_or_default(),
                name: proj
                    .as_ref()
                    .map(|p| p.title.clone())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| v.name.clone()),
                version_id: v.id.clone(),
                version_number: v.version_number.clone(),
                icon_url: proj.as_ref().and_then(|p| p.icon_url.clone()),
            });
            if let Some(p) = proj {
                file.project_type = match p.project_type.as_str() {
                    "mod" if p.is_datapack() => ProjectType::Datapack,
                    other => ProjectType::from_api(other),
                };
            }
        } else if let Some((mod_id, cfile)) = cf_found.get(&(prints[i] as u64)) {
            let info = cf.get_mod(*mod_id).await.ok();
            file.matched = Some(DroppedMatch {
                provider: ProviderId::Curseforge,
                project_id: mod_id.to_string(),
                slug: info.as_ref().map(|m| m.slug.clone()).unwrap_or_default(),
                name: info
                    .as_ref()
                    .map(|m| m.name.clone())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| file.filename.clone()),
                version_id: cfile.id.to_string(),
                version_number: curseforge::file_version(cfile),
                icon_url: info
                    .as_ref()
                    .and_then(|m| m.logo.clone())
                    .map(|l| l.url)
                    .filter(|u| !u.is_empty()),
            });
            if let Some(m) = info {
                file.project_type = curseforge::type_from_class(m.class_id);
            }
        }
        if let Some(m) = &file.matched {
            file.already_in_pack = existing.contains(&m.project_id);
        }
    }
    if !failures.is_empty() {
        for file in out.iter_mut() {
            file.unchecked = file.matched.is_none();
        }
    }
    Ok(DroppedScan {
        files: out,
        failures,
    })
}

pub fn remember_unchecked(pack_dir: &Path, items: &[DroppedFile]) -> Result<()> {
    let folders = Folders::load(pack_dir);
    let mut local = packlocal::read(pack_dir);
    let before = local.unchecked.len();
    for item in items.iter().filter(|i| i.unchecked && i.matched.is_none()) {
        let rel = format!(
            "{}/{}/{}",
            crate::instance::OVERRIDES_DIR,
            folders.install_dir(item.project_type),
            item.filename
        );
        if !local.unchecked.contains(&rel) {
            local.unchecked.push(rel);
        }
    }
    if local.unchecked.len() != before {
        packlocal::write(pack_dir, &local)?;
    }
    Ok(())
}

pub struct Recheck {
    pub found: Vec<DroppedFile>,
    pub paths: Vec<PathBuf>,
    pub pending: Vec<String>,
}

pub async fn recheck(mr: &Modrinth, cf: &CurseForge, pack_dir: &Path) -> Result<Recheck> {
    let listed = packlocal::read(pack_dir).unchecked;
    let mut out = Recheck {
        found: Vec::new(),
        paths: Vec::new(),
        pending: Vec::new(),
    };
    let by_path: HashMap<String, String> = listed
        .iter()
        .filter(|rel| pack_dir.join(rel).is_file())
        .map(|rel| {
            (
                pack_dir.join(rel).to_string_lossy().into_owned(),
                rel.clone(),
            )
        })
        .collect();
    if by_path.is_empty() {
        return Ok(out);
    }
    let files: Vec<String> = by_path.keys().cloned().collect();
    let scan = identify(mr, cf, pack_dir, &files).await?;
    let mut seen: HashSet<String> = HashSet::new();
    for file in scan.files {
        let Some(rel) = by_path.get(&file.path) else {
            continue;
        };
        seen.insert(rel.clone());
        if file.matched.is_some() {
            if !file.already_in_pack {
                out.paths.push(PathBuf::from(&file.path));
                out.found.push(file);
            }
        } else if file.unchecked {
            out.pending.push(rel.clone());
        }
    }
    out.pending
        .extend(by_path.values().filter(|rel| !seen.contains(*rel)).cloned());
    out.pending.sort();
    Ok(out)
}

pub fn set_unchecked(pack_dir: &Path, pending: Vec<String>) -> Result<()> {
    let mut local = packlocal::read(pack_dir);
    if local.unchecked == pending {
        return Ok(());
    }
    local.unchecked = pending;
    packlocal::write(pack_dir, &local)
}

pub fn add(pack_dir: &Path, items: &[DroppedFile]) -> Result<()> {
    for item in items {
        match &item.matched {
            Some(m) if !item.already_in_pack => {
                let mut sources = BTreeMap::new();
                sources.insert(
                    m.provider,
                    SourceFile {
                        id: Some(m.project_id.clone()),
                        slug: m.slug.clone(),
                        version_id: m.version_id.clone(),
                        ..Default::default()
                    },
                );
                content::add_item(
                    pack_dir,
                    &ContentItem {
                        name: m.name.clone(),
                        project_type: item.project_type,
                        file: String::new(),
                        side: "auto".into(),
                        explicit: true,
                        disabled: false,
                        pinned: false,
                        preferred: m.provider,
                        sources,
                        dependents: Vec::new(),
                        optional_dependents: Vec::new(),
                        no_bundle: false,
                        client_side: "required".into(),
                        server_side: "required".into(),
                    },
                )?;
            }
            Some(_) => {}
            None => {
                let dest_dir = crate::instance::overrides_dir(pack_dir)
                    .join(Folders::load(pack_dir).install_dir(item.project_type));
                std::fs::create_dir_all(&dest_dir)?;
                std::fs::copy(&item.path, dest_dir.join(&item.filename))?;
            }
        }
    }
    Ok(())
}
