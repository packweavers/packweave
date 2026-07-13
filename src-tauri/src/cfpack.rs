use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::curseforge::{self, CurseForge};
use crate::instance;
use crate::lockfile::{target_dir, Lockfile};
use crate::manifest::Manifest;
use crate::providers::ProviderId;
use crate::ptype::ProjectType;

struct CfEntry {
    project_id: u64,
    file_id: u64,
    name: String,
    is_mod: bool,
}

pub fn export_selfupdate(
    manifest: &Manifest,
    output: &Path,
    url: &str,
    installer: &[u8],
    installer_name: &str,
) -> Result<()> {
    let cf_manifest = manifest_json(
        &manifest.minecraft,
        &manifest.loader,
        manifest.loader_version.as_deref(),
        &manifest.name,
        &manifest.version,
        &[],
    );

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
    cf: &CurseForge,
    lock: &Lockfile,
    pack_name: &str,
    version: &str,
    output: &Path,
    env: &str,
    overrides_dir: &Path,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("packweave/0.1.1")
        .build()?;
    let mut entries: Vec<CfEntry> = Vec::new();
    let mut pending: Vec<(String, String, bool, Vec<u8>, u32)> = Vec::new();

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
                    entries.push(CfEntry {
                        project_id: p,
                        file_id: f,
                        name: m.name.clone(),
                        is_mod: m.project_type == ProjectType::Mod,
                    });
                    continue;
                }
            }
        }
        let src = match m.active() {
            Some(s) if !s.download_url.is_empty() => s,
            _ => continue,
        };
        let bytes = client
            .get(&src.download_url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .with_context(|| format!("downloading {}", m.name))?
            .bytes()
            .await
            .with_context(|| format!("downloading {}", m.name))?;
        let rel = format!("{}/{}", target_dir(m.project_type), src.filename);
        let print = curseforge::cf_fingerprint(&bytes);
        pending.push((
            m.name.clone(),
            rel,
            m.project_type == ProjectType::Mod,
            bytes.to_vec(),
            print,
        ));
    }

    let prints: Vec<u32> = pending.iter().map(|p| p.4).collect();
    let matched = cf.fingerprints(&prints).await.unwrap_or_default();
    let mut bundled: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, rel, is_mod, bytes, print) in pending {
        match matched.get(&(print as u64)) {
            Some((mod_id, file)) => entries.push(CfEntry {
                project_id: *mod_id as u64,
                file_id: file.id as u64,
                name,
                is_mod,
            }),
            None => bundled.push((rel, bytes)),
        }
    }

    let files: Vec<Value> = entries
        .iter()
        .map(|e| {
            json!({
                "projectID": e.project_id,
                "fileID": e.file_id,
                "required": true,
            })
        })
        .collect();
    let manifest = manifest_json(
        &lock.minecraft,
        &lock.loader,
        lock.loader_version.as_deref(),
        pack_name,
        version,
        &files,
    );

    let file = std::fs::File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    zip.start_file("modlist.html", options)?;
    zip.write_all(modlist_html(&entries).as_bytes())?;

    let mut written: HashSet<String> = HashSet::new();
    for (rel, bytes) in &bundled {
        let name = format!("overrides/{rel}");
        if !written.insert(name.clone()) {
            continue;
        }
        zip.start_file(name, options)?;
        zip.write_all(bytes)?;
    }
    if overrides_dir.is_dir() {
        add_overrides(
            &mut zip,
            overrides_dir,
            overrides_dir,
            options,
            &mut written,
        )?;
    }
    zip.finish()?;
    Ok(())
}

fn manifest_json(
    minecraft: &str,
    loader: &str,
    loader_version: Option<&str>,
    name: &str,
    version: &str,
    files: &[Value],
) -> Value {
    let mut mc = Map::new();
    mc.insert("version".into(), json!(minecraft));
    let loaders = match loader_id(loader, minecraft, loader_version) {
        Some(id) => json!([{ "id": id, "primary": true }]),
        None => json!([]),
    };
    mc.insert("modLoaders".into(), loaders);
    json!({
        "minecraft": Value::Object(mc),
        "manifestType": "minecraftModpack",
        "manifestVersion": 1,
        "name": name,
        "version": version,
        "author": "",
        "files": files,
        "overrides": "overrides",
    })
}

fn loader_id(
    loader: &str,
    minecraft: &str,
    version: Option<&str>,
) -> Option<String> {
    let prefix = match loader {
        "fabric" => "fabric",
        "quilt" => "quilt",
        "forge" => "forge",
        "neoforge" => "neoforge",
        _ => return None,
    };
    let version = version.unwrap_or_default();
    if version.is_empty() {
        return Some(prefix.to_string());
    }
    if loader == "neoforge" && minecraft == "1.20.1" {
        return Some(format!("neoforge-1.20.1-{version}"));
    }
    Some(format!("{prefix}-{version}"))
}

fn modlist_html(entries: &[CfEntry]) -> String {
    let mut out = String::from("<ul>");
    for e in entries.iter().filter(|e| e.is_mod) {
        out.push_str(&format!(
            "<li><a href=\"https://www.curseforge.com/projects/{}\">{}</a></li>\n",
            e.project_id,
            html_escape(&e.name)
        ));
    }
    out.push_str("</ul>");
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn add_overrides(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    options: SimpleFileOptions,
    written: &mut HashSet<String>,
) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            add_overrides(zip, base, &p, options, written)?;
        } else if p.is_file() {
            let rel = match p.strip_prefix(base) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if instance::is_os_junk(&rel) {
                continue;
            }
            let name = format!("overrides/{rel}");
            if !written.insert(name.clone()) {
                continue;
            }
            let bytes = std::fs::read(&p)?;
            zip.start_file(name, options)?;
            zip.write_all(&bytes)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_ids_match_curseforge_format() {
        assert_eq!(
            loader_id("fabric", "1.21.1", Some("0.16.5")).as_deref(),
            Some("fabric-0.16.5")
        );
        assert_eq!(
            loader_id("neoforge", "1.21.1", Some("21.1.72")).as_deref(),
            Some("neoforge-21.1.72")
        );
        assert_eq!(
            loader_id("neoforge", "1.20.1", Some("47.1.106")).as_deref(),
            Some("neoforge-1.20.1-47.1.106")
        );
        assert_eq!(loader_id("vanilla", "1.21.1", None), None);
    }

    #[test]
    fn modlist_escapes_names() {
        let entries = vec![CfEntry {
            project_id: 238222,
            file_id: 1,
            name: "Just Enough Items <JEI> & Co".into(),
            is_mod: true,
        }];
        let html = modlist_html(&entries);
        assert!(html.contains("https://www.curseforge.com/projects/238222"));
        assert!(html.contains("Just Enough Items &lt;JEI&gt; &amp; Co"));
    }
}
