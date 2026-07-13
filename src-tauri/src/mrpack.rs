use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::instance;
use crate::lockfile::{target_dir, LockedMod, Lockfile};
use crate::manifest::Manifest;
use crate::providers::ProviderId;

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

fn file_entry(
    path: &str,
    url: &str,
    sha1: &str,
    sha512: &str,
    size: u64,
    m: &LockedMod,
) -> Value {
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

pub async fn export(
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

    let mut files: Vec<Value> = Vec::new();
    let mut bundled: Vec<(String, Vec<u8>)> = Vec::new();

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
                Some(s) if !s.download_url.is_empty() => s,
                _ => continue,
            },
        };
        let path = format!("{}/{}", target_dir(m.project_type), src.filename);
        let allowed = allowed_url(&src.download_url);

        let sha1 = src.sha1.as_deref().filter(|s| !s.is_empty());
        let sha512 = src.sha512.as_deref().filter(|s| !s.is_empty());
        if let (Some(url), Some(sha1), Some(sha512)) =
            (allowed.as_deref(), sha1, sha512)
        {
            if src.file_size > 0 {
                files.push(file_entry(
                    &path,
                    url,
                    sha1,
                    sha512,
                    src.file_size,
                    m,
                ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_whitelisted_hosts_pass() {
        assert!(allowed_url(
            "https://cdn.modrinth.com/data/AANobbMI/versions/x/sodium.jar"
        )
        .is_some());
        assert!(allowed_url(
            "https://github.com/you/repo/releases/download/v1/a.jar"
        )
        .is_some());
        assert!(allowed_url(
            "https://raw.githubusercontent.com/you/repo/main/a.jar"
        )
        .is_some());
        assert!(allowed_url("https://gitlab.com/you/repo/-/raw/main/a.jar")
            .is_some());
        assert!(allowed_url("https://edge.forgecdn.net/files/123/456/a.jar")
            .is_none());
        assert!(
            allowed_url("https://mediafilez.forgecdn.net/files/1/2/a.jar")
                .is_none()
        );
        assert!(allowed_url("https://example.com/a.jar").is_none());
        assert!(allowed_url("not a url").is_none());
    }

    #[test]
    fn allowed_url_normalizes() {
        let u = allowed_url(
            "https://github.com/you/repo/releases/download/v1/my mod.jar",
        )
        .unwrap();
        assert_eq!(
            u,
            "https://github.com/you/repo/releases/download/v1/my%20mod.jar"
        );
    }

    #[tokio::test]
    async fn export_writes_spec_compliant_index() {
        use crate::lockfile::{DepType, SourceFile};
        use std::collections::BTreeMap;
        use std::io::Read;

        let mut sources = BTreeMap::new();
        sources.insert(
            ProviderId::Modrinth,
            SourceFile {
                id: Some("AANobbMI".into()),
                slug: "sodium".into(),
                version_id: "abc123".into(),
                version_number: "0.6.0".into(),
                filename: "sodium-fabric-0.6.0.jar".into(),
                download_url:
                    "https://cdn.modrinth.com/data/AANobbMI/versions/abc123/sodium-fabric-0.6.0.jar"
                        .into(),
                sha1: Some("a1".into()),
                sha512: Some("a512".into()),
                file_size: 1234,
                ..Default::default()
            },
        );
        let lock = Lockfile {
            minecraft: "1.21.1".into(),
            loader: "fabric".into(),
            loader_version: Some("0.16.5".into()),
            mods: vec![LockedMod {
                name: "Sodium".into(),
                project_id: "AANobbMI".into(),
                slug: "sodium".into(),
                project_type: crate::ptype::ProjectType::Mod,
                preferred: ProviderId::Modrinth,
                sources,
                dependency_type: DepType::Explicit,
                dependents: vec![],
                client_side: "required".into(),
                server_side: "optional".into(),
                disabled: false,
            }],
        };

        let out = std::env::temp_dir().join("packweave-export-test.mrpack");
        export(
            &lock,
            "Test Pack",
            "1.0.0",
            &out,
            "common",
            Path::new("/nonexistent"),
        )
        .await
        .unwrap();

        let mut zip =
            zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        let mut text = String::new();
        zip.by_name("modrinth.index.json")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        let _ = std::fs::remove_file(&out);

        let index: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(index["formatVersion"], 1);
        assert_eq!(index["game"], "minecraft");
        assert_eq!(index["versionId"], "1.0.0");
        assert_eq!(index["dependencies"]["minecraft"], "1.21.1");
        assert_eq!(index["dependencies"]["fabric-loader"], "0.16.5");
        let f = &index["files"][0];
        assert_eq!(f["path"], "mods/sodium-fabric-0.6.0.jar");
        assert_eq!(f["hashes"]["sha1"], "a1");
        assert_eq!(f["hashes"]["sha512"], "a512");
        assert_eq!(f["fileSize"], 1234);
        assert_eq!(f["env"]["client"], "required");
        assert_eq!(f["env"]["server"], "optional");
        assert_eq!(
            f["downloads"][0],
            "https://cdn.modrinth.com/data/AANobbMI/versions/abc123/sodium-fabric-0.6.0.jar"
        );
    }
}
