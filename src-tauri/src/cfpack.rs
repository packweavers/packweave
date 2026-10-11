use anyhow::{anyhow, Context, Result};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::curseforge::{self, CurseForge};
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::packignore::PackFile;
use crate::providers::ProviderId;
use crate::ptype::{Folders, ProjectType};
use crate::publish::Bundling;

struct CfEntry {
    project_id: u64,
    file_id: u64,
    name: String,
    is_mod: bool,
}

struct Bundled {
    name: String,
    rel: String,
    bytes: Vec<u8>,
}

struct Candidate {
    name: String,
    rel: String,
    is_mod: bool,
    bytes: Vec<u8>,
    print: u32,
    no_bundle: bool,
}

struct Plan {
    entries: Vec<CfEntry>,
    bundled: Vec<Bundled>,
    left_out: Vec<String>,
}

pub fn export_selfupdate(
    manifest: &Manifest,
    output: &Path,
    url: &str,
    installer: &[u8],
    installer_name: &str,
    token: Option<&str>,
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
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&cf_manifest)?.as_bytes())?;
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
    zip.finish()?;
    Ok(())
}

async fn plan(cf: &CurseForge, lock: &Lockfile, env: &str, folders: &Folders) -> Result<Plan> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()?;
    let mut plan = Plan {
        entries: Vec::new(),
        bundled: Vec::new(),
        left_out: Vec::new(),
    };
    let mut pending: Vec<Candidate> = Vec::new();

    for m in &lock.mods {
        if m.unsupported_on(env) {
            continue;
        }
        let datapack = m.project_type == ProjectType::Datapack;
        let src = m.active().filter(|s| !s.download_url.is_empty());
        let listed = m.sources.get(&ProviderId::Curseforge).and_then(|cfs| {
            let project = cfs.id.as_ref()?.parse::<u64>().ok()?;
            let file = cfs.version_id.parse::<u64>().ok()?;
            Some((project, file))
        });
        if let Some((project_id, file_id)) = listed.filter(|_| !datapack || src.is_none()) {
            plan.entries.push(CfEntry {
                project_id,
                file_id,
                name: m.name.clone(),
                is_mod: m.project_type == ProjectType::Mod,
            });
            continue;
        }
        let Some(src) = src else {
            plan.left_out.push(m.name.clone());
            continue;
        };
        if datapack && m.no_bundle {
            plan.left_out.push(m.name.clone());
            continue;
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
        let rel = format!("{}/{}", folders.install_dir(m.project_type), src.filename);
        if datapack {
            plan.bundled.push(Bundled {
                name: m.name.clone(),
                rel,
                bytes: bytes.to_vec(),
            });
            continue;
        }
        pending.push(Candidate {
            name: m.name.clone(),
            rel,
            is_mod: m.project_type == ProjectType::Mod,
            print: curseforge::cf_fingerprint(&bytes),
            bytes: bytes.to_vec(),
            no_bundle: m.no_bundle,
        });
    }

    let prints: Vec<u32> = pending.iter().map(|c| c.print).collect();
    let matched = cf
        .fingerprints(&prints)
        .await
        .map_err(|e| anyhow!("Couldn't match files against CurseForge: {e}"))?;
    for c in pending {
        match matched.get(&(c.print as u64)) {
            Some((mod_id, file)) => plan.entries.push(CfEntry {
                project_id: *mod_id as u64,
                file_id: file.id as u64,
                name: c.name,
                is_mod: c.is_mod,
            }),
            None if c.no_bundle => plan.left_out.push(c.name),
            None => plan.bundled.push(Bundled {
                name: c.name,
                rel: c.rel,
                bytes: c.bytes,
            }),
        }
    }
    Ok(plan)
}

pub async fn bundling(
    cf: &CurseForge,
    lock: &Lockfile,
    env: &str,
    folders: &Folders,
) -> Result<Bundling> {
    let plan = plan(cf, lock, env, folders).await?;
    Ok(Bundling {
        bundled: plan.bundled.into_iter().map(|b| b.name).collect(),
        left_out: plan.left_out,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn export(
    cf: &CurseForge,
    lock: &Lockfile,
    pack_name: &str,
    version: &str,
    output: &Path,
    env: &str,
    overrides: &[PackFile],
    folders: &Folders,
) -> Result<()> {
    let Plan {
        entries, bundled, ..
    } = plan(cf, lock, env, folders).await?;
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
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    zip.start_file("modlist.html", options)?;
    zip.write_all(modlist_html(&entries).as_bytes())?;

    let mut written: HashSet<String> = HashSet::new();
    for b in &bundled {
        let name = format!("overrides/{}", b.rel);
        if !written.insert(name.clone()) {
            continue;
        }
        zip.start_file(name, options)?;
        zip.write_all(&b.bytes)?;
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

fn loader_id(loader: &str, minecraft: &str, version: Option<&str>) -> Option<String> {
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
