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
    let loader_id =
        match (loader_cf(&manifest.loader), &manifest.loader_version) {
            (Some(prefix), Some(v)) => format!("{prefix}-{v}"),
            (Some(prefix), None) => prefix.to_string(),
            _ => String::new(),
        };
    let mut mc = Map::new();
    mc.insert("version".into(), json!(manifest.minecraft));
    let mut ml = Map::new();
    ml.insert("id".into(), json!(loader_id));
    ml.insert("primary".into(), json!(true));
    mc.insert("modLoaders".into(), json!([Value::Object(ml)]));

    let cf_manifest = json!({
        "minecraft": Value::Object(mc),
        "manifestType": "minecraftModpack",
        "manifestVersion": 1,
        "name": manifest.name,
        "version": manifest.version,
        "author": "",
        "files": [],
        "overrides": "overrides",
    });

    let file = std::fs::File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&cf_manifest)?.as_bytes())?;
    zip.start_file(format!("overrides/{installer_name}"), options)?;
    zip.write_all(installer)?;
    zip.start_file("overrides/modpack.url", options)?;
    zip.write_all(format!("{url}\n").as_bytes())?;
    zip.finish()?;
    Ok(())
}

pub async fn export(
    lock: &Lockfile,
    pack_name: &str,
    version: &str,
    output: &Path,
    env: &str,
    overrides_dir: &Path,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("packweave/0.1.0")
        .build()?;
    let mut cf_files: Vec<Value> = Vec::new();
    let mut downloaded: Vec<(String, Vec<u8>)> = Vec::new();

    for m in &lock.mods {
        if env == "server" && m.server_side == "unsupported" {
            continue;
        }
        if env == "client" && m.client_side == "unsupported" {
            continue;
        }
        if let Some(cfs) = m.sources.get(&ProviderId::Curseforge) {
            if let Some(pid) = &cfs.id {
                if let (Ok(p), Ok(f)) =
                    (pid.parse::<u64>(), cfs.version_id.parse::<u64>())
                {
                    cf_files.push(json!({ "projectID": p, "fileID": f, "required": true }));
                    continue;
                }
            }
        }
        let src = match m.active() {
            Some(s) if !s.download_url.is_empty() => s,
            _ => continue,
        };
        let bytes = client.get(&src.download_url).send().await?.bytes().await?;
        let rel = format!("{}/{}", target_dir(m.project_type), src.filename);
        downloaded.push((rel, bytes.to_vec()));
    }

    let loader_id = match (loader_cf(&lock.loader), &lock.loader_version) {
        (Some(prefix), Some(v)) => format!("{prefix}-{v}"),
        (Some(prefix), None) => prefix.to_string(),
        _ => String::new(),
    };
    let mut mc = Map::new();
    mc.insert("version".into(), json!(lock.minecraft));
    let mut ml = Map::new();
    ml.insert("id".into(), json!(loader_id));
    ml.insert("primary".into(), json!(true));
    mc.insert("modLoaders".into(), json!([Value::Object(ml)]));

    let manifest = json!({
        "minecraft": Value::Object(mc),
        "manifestType": "minecraftModpack",
        "manifestVersion": 1,
        "name": pack_name,
        "version": version,
        "author": "",
        "files": cf_files,
        "overrides": "overrides",
    });

    let file = std::fs::File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    for (rel, bytes) in &downloaded {
        zip.start_file(format!("overrides/{rel}"), options)?;
        zip.write_all(bytes)?;
    }
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

fn loader_cf(loader: &str) -> Option<&'static str> {
    match loader {
        "fabric" => Some("fabric"),
        "quilt" => Some("quilt"),
        "forge" => Some("forge"),
        "neoforge" => Some("neoforge"),
        _ => None,
    }
}
