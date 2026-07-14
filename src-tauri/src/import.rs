use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use crate::content::{self, ContentItem};
use crate::curseforge::{self, CurseForge};
use crate::instance;
use crate::lockfile::SourceFile;
use crate::manifest::{self, Manifest};
use crate::modrinth::Modrinth;
use crate::providers::ProviderId;
use crate::ptype::ProjectType;

type Archive = zip::ZipArchive<std::fs::File>;

pub async fn import_pack(
    mr: &Modrinth,
    cf: &CurseForge,
    src: &Path,
    dest: &Path,
) -> Result<Manifest> {
    let file = std::fs::File::open(src)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    if names.iter().any(|n| n == "modrinth.index.json") {
        import_mrpack(mr, &mut archive, dest).await
    } else if names.iter().any(|n| n == "manifest.json") {
        import_curseforge(&mut archive, dest)
    } else if names
        .iter()
        .any(|n| n == "instance.cfg" || n.ends_with("/instance.cfg"))
    {
        import_prism(mr, cf, &mut archive, dest).await
    } else {
        bail!("Unrecognized pack. Expected a Modrinth (.mrpack), CurseForge (.zip), or Prism / MultiMC instance (.zip).")
    }
}

async fn import_mrpack(
    mr: &Modrinth,
    archive: &mut Archive,
    dest: &Path,
) -> Result<Manifest> {
    let index: Value = read_json(archive, "modrinth.index.json")?;
    let name = index["name"]
        .as_str()
        .unwrap_or("Imported Pack")
        .to_string();
    let deps = index["dependencies"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let minecraft = deps
        .get("minecraft")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let mut loader = "vanilla".to_string();
    let mut loader_version: Option<String> = None;
    for (k, v) in &deps {
        let ver = v.as_str().map(|s| s.to_string());
        match k.as_str() {
            "fabric-loader" => {
                loader = "fabric".into();
                loader_version = ver;
            }
            "quilt-loader" => {
                loader = "quilt".into();
                loader_version = ver;
            }
            "forge" => {
                loader = "forge".into();
                loader_version = ver;
            }
            "neoforge" => {
                loader = "neoforge".into();
                loader_version = ver;
            }
            _ => {}
        }
    }
    let mf = Manifest {
        name,
        version: "1.0.0".into(),
        minecraft,
        loader,
        loader_version,
        channel: crate::manifest::Channel::Release,
    };
    std::fs::create_dir_all(dest)?;
    manifest::write(dest, &mf)?;

    let files = index["files"].as_array().cloned().unwrap_or_default();
    let mut sha_for_idx: Vec<Option<String>> = Vec::new();
    let mut hashes: Vec<String> = Vec::new();
    for f in &files {
        let sha1 = f["hashes"]["sha1"].as_str().map(|s| s.to_string());
        if let Some(s) = &sha1 {
            hashes.push(s.clone());
        }
        sha_for_idx.push(sha1);
    }
    let by_hash = mr.versions_by_hashes(&hashes).await.unwrap_or_default();
    let proj_ids: Vec<String> =
        by_hash.values().map(|v| v.project_id.clone()).collect();
    let projects = mr.projects_bulk(&proj_ids).await.unwrap_or_default();
    let mut name_by_pid: HashMap<String, (String, String)> = HashMap::new();
    for p in projects {
        name_by_pid.insert(p.id.clone(), (p.title.clone(), p.slug.clone()));
    }

    for (i, f) in files.iter().enumerate() {
        let path = f["path"].as_str().unwrap_or_default().to_string();
        let project_type = type_from_path(&path);
        let env_client = f["env"]["client"]
            .as_str()
            .unwrap_or("required")
            .to_string();
        let env_server = f["env"]["server"]
            .as_str()
            .unwrap_or("required")
            .to_string();
        let side = side_from_env(&env_client, &env_server);
        let matched = sha_for_idx[i].as_ref().and_then(|s| by_hash.get(s));
        if let Some(v) = matched {
            let (nm, slug) = name_by_pid
                .get(&v.project_id)
                .cloned()
                .unwrap_or_else(|| (file_stem(&path), v.project_id.clone()));
            let mut sources = BTreeMap::new();
            sources.insert(
                ProviderId::Modrinth,
                SourceFile {
                    id: Some(v.project_id.clone()),
                    pin: Some(v.id.clone()),
                    slug,
                    version_id: v.id.clone(),
                    version_number: v.version_number.clone(),
                    ..Default::default()
                },
            );
            content::add_item(
                dest,
                &ContentItem {
                    name: nm,
                    project_type,
                    side,
                    explicit: true,
                    disabled: false,
                    preferred: ProviderId::Modrinth,
                    sources,
                    dependents: Vec::new(),
                    client_side: env_client,
                    server_side: env_server,
                },
            )?;
        } else {
            let url =
                f["downloads"][0].as_str().unwrap_or_default().to_string();
            if url.is_empty() {
                continue;
            }
            let stem = file_stem(&path);
            let mut sources = BTreeMap::new();
            sources.insert(
                ProviderId::Url,
                SourceFile {
                    url: Some(url),
                    slug: stem.clone(),
                    ..Default::default()
                },
            );
            content::add_item(
                dest,
                &ContentItem {
                    name: stem,
                    project_type,
                    side,
                    explicit: true,
                    disabled: false,
                    preferred: ProviderId::Url,
                    sources,
                    dependents: Vec::new(),
                    client_side: env_client,
                    server_side: env_server,
                },
            )?;
        }
    }

    extract_overrides(
        archive,
        dest,
        &["overrides/", "client-overrides/", "server-overrides/"],
    )?;
    Ok(mf)
}

fn import_curseforge(archive: &mut Archive, dest: &Path) -> Result<Manifest> {
    let mj: Value = read_json(archive, "manifest.json")?;
    let name = mj["name"].as_str().unwrap_or("Imported Pack").to_string();
    let minecraft = mj["minecraft"]["version"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let loaders = mj["minecraft"]["modLoaders"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let primary = loaders
        .iter()
        .find(|l| l["primary"].as_bool().unwrap_or(false))
        .or_else(|| loaders.first());
    let (loader, loader_version) = match primary.and_then(|l| l["id"].as_str())
    {
        Some(id) => parse_cf_loader(id),
        None => ("vanilla".to_string(), None),
    };
    let mf = Manifest {
        name,
        version: "1.0.0".into(),
        minecraft,
        loader,
        loader_version,
        channel: crate::manifest::Channel::Release,
    };
    std::fs::create_dir_all(dest)?;
    manifest::write(dest, &mf)?;

    let files = mj["files"].as_array().cloned().unwrap_or_default();
    for f in &files {
        let pid = f["projectID"].as_i64().map(|n| n.to_string());
        let fid = f["fileID"].as_i64().map(|n| n.to_string());
        let (pid, fid) = match (pid, fid) {
            (Some(p), Some(f)) => (p, f),
            _ => continue,
        };
        let mut sources = BTreeMap::new();
        sources.insert(
            ProviderId::Curseforge,
            SourceFile {
                id: Some(pid.clone()),
                pin: Some(fid),
                slug: pid,
                ..Default::default()
            },
        );
        content::add_item(
            dest,
            &ContentItem {
                name: String::new(),
                project_type: ProjectType::Mod,
                side: "auto".into(),
                explicit: true,
                disabled: false,
                preferred: ProviderId::Curseforge,
                sources,
                dependents: Vec::new(),
                client_side: "required".into(),
                server_side: "required".into(),
            },
        )?;
    }

    let ov = mj["overrides"].as_str().unwrap_or("overrides");
    extract_overrides(archive, dest, &[&format!("{ov}/")])?;
    Ok(mf)
}

fn read_json(archive: &mut Archive, name: &str) -> Result<Value> {
    let mut f = archive.by_name(name)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(serde_json::from_str(&s)?)
}

fn read_text(archive: &mut Archive, name: &str) -> Result<String> {
    let mut f = archive.by_name(name)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

fn read_bytes(archive: &mut Archive, name: &str) -> Result<Vec<u8>> {
    let mut f = archive.by_name(name)?;
    let mut b = Vec::new();
    f.read_to_end(&mut b)?;
    Ok(b)
}

async fn import_prism(
    mr: &Modrinth,
    cf: &CurseForge,
    archive: &mut Archive,
    dest: &Path,
) -> Result<Manifest> {
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    let cfg_name = names
        .iter()
        .find(|n| n.as_str() == "instance.cfg" || n.ends_with("/instance.cfg"))
        .cloned()
        .ok_or_else(|| anyhow!("Not a Prism / MultiMC instance."))?;
    let root = cfg_name
        .strip_suffix("instance.cfg")
        .unwrap_or("")
        .to_string();

    let mut name = "Imported Pack".to_string();
    if let Ok(cfg) = read_text(archive, &cfg_name) {
        for line in cfg.lines() {
            if let Some(v) = line.strip_prefix("name=") {
                if !v.trim().is_empty() {
                    name = v.trim().to_string();
                }
            }
        }
    }

    let mut minecraft = String::new();
    let mut loader = "vanilla".to_string();
    let mut loader_version: Option<String> = None;
    if let Ok(text) = read_text(archive, &format!("{root}mmc-pack.json")) {
        if let Ok(json) = serde_json::from_str::<Value>(&text) {
            let comps =
                json.get("components").and_then(|c| c.as_array()).cloned();
            for comp in comps.unwrap_or_default() {
                let uid =
                    comp.get("uid").and_then(|u| u.as_str()).unwrap_or("");
                let ver = comp
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                match uid {
                    "net.minecraft" => minecraft = ver.unwrap_or_default(),
                    "net.fabricmc.fabric-loader" => {
                        loader = "fabric".into();
                        loader_version = ver;
                    }
                    "org.quiltmc.quilt-loader" => {
                        loader = "quilt".into();
                        loader_version = ver;
                    }
                    "net.minecraftforge" => {
                        loader = "forge".into();
                        loader_version = ver;
                    }
                    "net.neoforged" => {
                        loader = "neoforge".into();
                        loader_version = ver;
                    }
                    _ => {}
                }
            }
        }
    }

    let mf = Manifest {
        name,
        version: "1.0.0".into(),
        minecraft,
        loader,
        loader_version,
        channel: crate::manifest::Channel::Release,
    };
    std::fs::create_dir_all(dest)?;
    manifest::write(dest, &mf)?;

    let game = [".minecraft/", "minecraft/"]
        .iter()
        .map(|g| format!("{root}{g}"))
        .find(|g| names.iter().any(|n| n.starts_with(g.as_str())))
        .unwrap_or_else(|| format!("{root}.minecraft/"));

    let mod_names: Vec<String> = names
        .iter()
        .filter(|n| match n.strip_prefix(game.as_str()) {
            Some(r) => {
                r.starts_with("mods/")
                    && (r.ends_with(".jar") || r.ends_with(".zip"))
            }
            None => false,
        })
        .cloned()
        .collect();

    let mut jars: Vec<(String, String, u32)> = Vec::new();
    for n in &mod_names {
        if let Ok(bytes) = read_bytes(archive, n) {
            let mut h = Sha1::new();
            h.update(&bytes);
            let sha1 = hex::encode(h.finalize());
            let murmur = curseforge::cf_fingerprint(&bytes);
            jars.push((n.clone(), sha1, murmur));
        }
    }

    let sha1s: Vec<String> = jars.iter().map(|j| j.1.clone()).collect();
    let mr_found = mr.versions_by_hashes(&sha1s).await.unwrap_or_default();
    let prints: Vec<u32> = jars
        .iter()
        .filter(|j| !mr_found.contains_key(&j.1))
        .map(|j| j.2)
        .collect();
    let cf_found = cf.fingerprints(&prints).await.unwrap_or_default();

    let proj_ids: Vec<String> =
        mr_found.values().map(|v| v.project_id.clone()).collect();
    let projects = mr.projects_bulk(&proj_ids).await.unwrap_or_default();
    let name_by_pid: HashMap<String, (String, String)> = projects
        .into_iter()
        .map(|p| (p.id.clone(), (p.title, p.slug)))
        .collect();

    let mut matched: HashSet<String> = HashSet::new();
    for (entry, sha1, murmur) in &jars {
        if let Some(v) = mr_found.get(sha1) {
            let (nm, slug) = name_by_pid
                .get(&v.project_id)
                .cloned()
                .unwrap_or_else(|| (file_stem(entry), v.project_id.clone()));
            let mut sources = BTreeMap::new();
            sources.insert(
                ProviderId::Modrinth,
                SourceFile {
                    id: Some(v.project_id.clone()),
                    pin: Some(v.id.clone()),
                    slug,
                    version_id: v.id.clone(),
                    version_number: v.version_number.clone(),
                    ..Default::default()
                },
            );
            content::add_item(
                dest,
                &mod_item(nm, ProviderId::Modrinth, sources),
            )?;
            matched.insert(entry.clone());
        } else if let Some((mod_id, file)) = cf_found.get(&(*murmur as u64)) {
            let mut sources = BTreeMap::new();
            sources.insert(
                ProviderId::Curseforge,
                SourceFile {
                    id: Some(mod_id.to_string()),
                    pin: Some(file.id.to_string()),
                    slug: mod_id.to_string(),
                    ..Default::default()
                },
            );
            content::add_item(
                dest,
                &mod_item(file_stem(entry), ProviderId::Curseforge, sources),
            )?;
            matched.insert(entry.clone());
        }
    }

    let base = instance::overrides_dir(dest);
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let full = entry.name().to_string();
        let rel = match full.strip_prefix(game.as_str()) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        if matched.contains(&full) || rel.ends_with(".disabled") {
            continue;
        }
        let out = base.join(rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        std::fs::write(out, buf)?;
    }
    Ok(mf)
}

fn mod_item(
    name: String,
    provider: ProviderId,
    sources: BTreeMap<ProviderId, SourceFile>,
) -> ContentItem {
    ContentItem {
        name,
        project_type: ProjectType::Mod,
        side: "auto".into(),
        explicit: true,
        disabled: false,
        preferred: provider,
        sources,
        dependents: Vec::new(),
        client_side: "required".into(),
        server_side: "required".into(),
    }
}

fn extract_overrides(
    archive: &mut Archive,
    dest: &Path,
    prefixes: &[&str],
) -> Result<()> {
    let base = instance::overrides_dir(dest);
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let rel = prefixes.iter().find_map(|p| name.strip_prefix(p));
        let rel = match rel {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        let out = base.join(rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        std::fs::write(out, buf)?;
    }
    Ok(())
}

fn type_from_path(path: &str) -> ProjectType {
    match path.split('/').next().unwrap_or("") {
        "resourcepacks" => ProjectType::Resourcepack,
        "shaderpacks" | "shaders" => ProjectType::Shader,
        _ => ProjectType::Mod,
    }
}

fn side_from_env(client: &str, server: &str) -> String {
    if client == "unsupported" {
        "server".into()
    } else if server == "unsupported" {
        "client".into()
    } else {
        "auto".into()
    }
}

fn file_stem(path: &str) -> String {
    let base = path.rsplit('/').next().unwrap_or(path);
    base.rsplit_once('.')
        .map(|(s, _)| s.to_string())
        .unwrap_or_else(|| base.to_string())
}

fn parse_cf_loader(id: &str) -> (String, Option<String>) {
    let (prefix, ver) = id.split_once('-').unwrap_or((id, ""));
    let loader = match prefix.to_lowercase().as_str() {
        "forge" => "forge",
        "fabric" => "fabric",
        "neoforge" => "neoforge",
        "quilt" => "quilt",
        other => return (other.to_string(), none_if_empty(ver)),
    };
    (loader.to_string(), none_if_empty(ver))
}

fn none_if_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
