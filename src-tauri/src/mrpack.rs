use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::lockfile::{LockedMod, Lockfile, SourceFile};
use crate::manifest::Manifest;
use crate::packignore::PackFile;
use crate::providers::ProviderId;
use crate::ptype::Folders;
use crate::publish::Bundling;

const ALLOWED_HOSTS: &[&str] = &[
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];

fn allowed_url(raw: &str) -> Option<String> {
    let url = reqwest::Url::parse(raw).ok()?;
    let host = url.host_str()?.to_lowercase();
    if ALLOWED_HOSTS.contains(&host.as_str()) {
        Some(url.to_string())
    } else {
        None
    }
}

fn file_entry(path: &str, url: &str, sha1: &str, sha512: &str, size: u64, m: &LockedMod) -> Value {
    json!({
        "path": path,
        "hashes": { "sha1": sha1, "sha512": sha512 },
        "env": {
            "client": env_value(&m.client_side),
            "server": env_value(&m.server_side),
        },
        "downloads": [url],
        "fileSize": size,
    })
}

pub fn export_selfupdate(
    manifest: &Manifest,
    output: &Path,
    url: &str,
    overrides_dir: &Path,
    installer: &[u8],
    installer_name: &str,
    token: Option<&str>,
) -> Result<()> {
    let mut deps = Map::new();
    deps.insert("minecraft".into(), json!(manifest.minecraft));
    if let (Some(key), Some(version)) = (loader_key(&manifest.loader), &manifest.loader_version) {
        deps.insert(key.into(), json!(version));
    }
    let index = json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": manifest.version,
        "name": manifest.name,
        "files": [],
        "dependencies": Value::Object(deps),
    });

    let file = std::fs::File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("modrinth.index.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;
    for folder in ["overrides", "overrides/mods"] {
        zip.start_file(format!("{folder}/{installer_name}"), options)?;
        zip.write_all(installer)?;
    }
    zip.start_file("overrides/modpack.url", options)?;
    zip.write_all(format!("{}\n", url.trim_end_matches('/')).as_bytes())?;
    if let Some(t) = token.filter(|t| !t.is_empty()) {
        zip.start_file("overrides/modpack.token", options)?;
        zip.write_all(format!("{t}\n").as_bytes())?;
    }
    if let Ok(icon) = std::fs::read(overrides_dir.join("icon.png")) {
        zip.start_file("overrides/icon.png", options)?;
        zip.write_all(&icon)?;
    }
    zip.finish()?;
    Ok(())
}

pub async fn export(
    lock: &Lockfile,
    pack_name: &str,
    version: &str,
    output: &Path,
    env: &str,
    overrides: &[PackFile],
    folders: &Folders,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()?;

    let mut files: Vec<Value> = Vec::new();
    let mut bundled: Vec<(String, Vec<u8>)> = Vec::new();

    for m in &lock.mods {
        if m.unsupported_on(env) {
            continue;
        }
        let (src, allowed) = match placement(m) {
            Placement::LeftOut => continue,
            Placement::Linked(src, url) => (src, Some(url)),
            Placement::Bundled(src) => (src, None),
        };
        let path = format!("{}/{}", folders.install_dir(m.project_type), src.filename);

        let sha1 = src.sha1.as_deref().filter(|s| !s.is_empty());
        let sha512 = src.sha512.as_deref().filter(|s| !s.is_empty());
        if let (Some(url), Some(sha1), Some(sha512)) = (allowed.as_deref(), sha1, sha512) {
            if src.file_size > 0 {
                files.push(file_entry(&path, url, sha1, sha512, src.file_size, m));
                continue;
            }
        }

        let bytes = client
            .get(&src.download_url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .with_context(|| format!("downloading {}", m.name))?
            .bytes()
            .await
            .with_context(|| format!("downloading {}", m.name))?;

        match allowed {
            Some(url) => {
                let sha1 = hex::encode(Sha1::digest(&bytes));
                let sha512 = hex::encode(Sha512::digest(&bytes));
                files.push(file_entry(
                    &path,
                    &url,
                    &sha1,
                    &sha512,
                    bytes.len() as u64,
                    m,
                ));
            }
            None => bundled.push((path, bytes.to_vec())),
        }
    }

    let mut deps = Map::new();
    deps.insert("minecraft".into(), json!(lock.minecraft));
    if let (Some(key), Some(version)) = (loader_key(&lock.loader), &lock.loader_version) {
        deps.insert(key.into(), json!(version));
    }

    let index = json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": version,
        "name": pack_name,
        "files": files,
        "dependencies": Value::Object(deps),
    });

    let file = std::fs::File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("modrinth.index.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;

    let mut written: HashSet<String> = HashSet::new();
    for (rel, bytes) in &bundled {
        let name = format!("overrides/{rel}");
        if !written.insert(name.clone()) {
            continue;
        }
        zip.start_file(name, options)?;
        zip.write_all(bytes)?;
    }

    for file in overrides {
        let name = format!("overrides/{}", file.rel);
        if !written.insert(name.clone()) {
            continue;
        }
        zip.start_file(name, options)?;
        zip.write_all(&std::fs::read(&file.path)?)?;
    }

    zip.finish()?;
    Ok(())
}

enum Placement<'a> {
    Linked(&'a SourceFile, String),
    Bundled(&'a SourceFile),
    LeftOut,
}

fn placement(m: &LockedMod) -> Placement<'_> {
    let src = match m.sources.get(&ProviderId::Modrinth) {
        Some(s) if !s.download_url.is_empty() => s,
        _ => match m.active() {
            Some(s) if !s.download_url.is_empty() => s,
            _ => return Placement::LeftOut,
        },
    };
    match allowed_url(&src.download_url) {
        Some(url) => Placement::Linked(src, url),
        None if m.no_bundle => Placement::LeftOut,
        None => Placement::Bundled(src),
    }
}

pub fn bundling(lock: &Lockfile, env: &str) -> Bundling {
    let mut out = Bundling::default();
    for m in lock.mods.iter().filter(|m| !m.unsupported_on(env)) {
        match placement(m) {
            Placement::Linked(..) => {}
            Placement::Bundled(_) => out.bundled.push(m.name.clone()),
            Placement::LeftOut => out.left_out.push(m.name.clone()),
        }
    }
    out
}

fn loader_key(loader: &str) -> Option<&'static str> {
    match loader {
        "fabric" => Some("fabric-loader"),
        "quilt" => Some("quilt-loader"),
        "forge" => Some("forge"),
        "neoforge" => Some("neoforge"),
        _ => None,
    }
}

fn env_value(side: &str) -> &str {
    match side {
        "required" => "required",
        "optional" => "optional",
        "unsupported" => "unsupported",
        _ => "required",
    }
}
