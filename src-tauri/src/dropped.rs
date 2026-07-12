use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::path::Path;

use crate::content::{self, ContentItem};
use crate::curseforge::{self, CurseForge};
use crate::lockfile::SourceFile;
use crate::modmeta;
use crate::modrinth::Modrinth;
use crate::providers::ProviderId;
use crate::ptype::ProjectType;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DroppedMatch {
    pub provider: ProviderId,
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub version_id: String,
    pub version_number: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DroppedFile {
    pub path: String,
    pub filename: String,
    pub project_type: ProjectType,
    pub already_in_pack: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched: Option<DroppedMatch>,
}

fn classify(path: &Path) -> ProjectType {
    if let Some(meta) = modmeta::read_archive(path) {
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

pub async fn identify(
    mr: &Modrinth,
    cf: &CurseForge,
    pack_dir: &Path,
    files: &[String],
) -> Result<Vec<DroppedFile>> {
    let existing: std::collections::HashSet<String> =
        content::read_all(pack_dir)
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
        });
    }

    let mr_found = mr.versions_from_hashes(&hashes).await.unwrap_or_default();
    let cf_found = cf.fingerprints(&prints).await.unwrap_or_default();

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
                file.project_type = ProjectType::from_api(&p.project_type);
            }
        } else if let Some((mod_id, cfile)) = cf_found.get(&(prints[i] as u64))
        {
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
    Ok(out)
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
                        pin: Some(m.version_id.clone()),
                        ..Default::default()
                    },
                );
                content::add_item(
                    pack_dir,
                    &ContentItem {
                        name: m.name.clone(),
                        project_type: item.project_type,
                        side: "auto".into(),
                        explicit: true,
                        disabled: false,
                        preferred: m.provider,
                        sources,
                        dependents: Vec::new(),
                        client_side: "required".into(),
                        server_side: "required".into(),
                    },
                )?;
            }
            Some(_) => {}
            None => {
                let dest_dir = crate::instance::overrides_dir(pack_dir)
                    .join(item.project_type.instance_dir());
                std::fs::create_dir_all(&dest_dir)?;
                std::fs::copy(&item.path, dest_dir.join(&item.filename))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_without_metadata_defaults_to_resourcepack() {
        assert_eq!(
            classify(Path::new("/tmp/does-not-exist.zip")),
            ProjectType::Resourcepack
        );
        assert_eq!(
            classify(Path::new("/tmp/does-not-exist.jar")),
            ProjectType::Mod
        );
    }
}
