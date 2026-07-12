use anyhow::Result;
use serde_json::{json, Map, Value};
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::lockfile::{target_dir, Lockfile};
use crate::manifest::Manifest;
use crate::providers::ProviderId;

pub fn export_selfupdate(
    manifest: &Manifest,
    output: &Path,
    url: &str,
    installer: &[u8],
    installer_name: &str,
) -> Result<()> {
    let mut deps = Map::new();
    deps.insert("minecraft".into(), json!(manifest.minecraft));
    if let (Some(key), Some(version)) =
        (loader_key(&manifest.loader), &manifest.loader_version)
    {
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
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    zip.start_file("modrinth.index.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;
    zip.start_file(format!("overrides/{installer_name}"), options)?;
    zip.write_all(installer)?;
    zip.start_file("overrides/modpack.url", options)?;
    zip.write_all(format!("{url}\n").as_bytes())?;
    zip.finish()?;
    Ok(())
}

pub fn export(
    lock: &Lockfile,
    pack_name: &str,
    version: &str,
    output: &Path,
    env: &str,
    overrides_dir: &Path,
) -> Result<()> {
    let mut files: Vec<Value> = Vec::new();
    for m in &lock.mods {
        if env == "server" && m.server_side == "unsupported" {
            continue;
        }
        if env == "client" && m.client_side == "unsupported" {
            continue;
        }
        let src = match m.sources.get(&ProviderId::Modrinth) {
            Some(s) if !s.download_url.is_empty() => s,
            _ => match m.active() {
                Some(s) => s,
                None => continue,
            },
        };
        let (filename, url, sha1, sha512, size) = (
            &src.filename,
            &src.download_url,
            &src.sha1,
            &src.sha512,
            src.file_size,
        );
        if url.is_empty() {
            continue;
        }
        let mut hashes = Map::new();
        if let Some(s) = sha1 {
            hashes.insert("sha1".into(), json!(s));
        }
        if let Some(s) = sha512 {
            hashes.insert("sha512".into(), json!(s));
        }
        files.push(json!({
            "path": format!("{}/{}", target_dir(m.project_type), filename),
            "hashes": hashes,
            "env": {
                "client": env_value(&m.client_side),
                "server": env_value(&m.server_side),
            },
            "downloads": [url],
            "fileSize": size,
        }));
    }

    let mut deps = Map::new();
    deps.insert("minecraft".into(), json!(lock.minecraft));
    if let (Some(key), Some(version)) =
        (loader_key(&lock.loader), &lock.loader_version)
    {
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
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    zip.start_file("modrinth.index.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&index)?.as_bytes())?;

    if overrides_dir.is_dir() {
        add_overrides(&mut zip, overrides_dir, overrides_dir, options)?;
    }

    zip.finish()?;
    Ok(())
}

fn add_overrides(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    options: SimpleFileOptions,
) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            add_overrides(zip, base, &p, options)?;
        } else if p.is_file() {
            let rel = match p.strip_prefix(base) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            let bytes = std::fs::read(&p)?;
            zip.start_file(format!("overrides/{rel}"), options)?;
            zip.write_all(&bytes)?;
        }
    }
    Ok(())
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
