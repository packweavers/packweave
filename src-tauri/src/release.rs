use anyhow::Result;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::dist::{INSTALLER, PACK_JAR};
use crate::git;
use crate::packignore::ExportFilter;

pub fn build_snapshot(
    pack_dir: &Path,
    pack_url: &str,
    dist_token: Option<&str>,
) -> Result<Vec<u8>> {
    build(pack_dir, None, pack_url, dist_token)
}

pub fn build_delta(
    pack_dir: &Path,
    prev_tag: &str,
    pack_url: &str,
    dist_token: Option<&str>,
) -> Option<Vec<u8>> {
    let changed: HashSet<String> = git::changed_paths_since(pack_dir, prev_tag, "overrides")
        .ok()?
        .into_iter()
        .collect();
    build(pack_dir, Some(&changed), pack_url, dist_token).ok()
}

fn build(
    pack_dir: &Path,
    only_overrides: Option<&HashSet<String>>,
    pack_url: &str,
    dist_token: Option<&str>,
) -> Result<Vec<u8>> {
    let filter = ExportFilter::load(pack_dir);
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for name in ["modpack.json", crate::preserve::FILE] {
        if let Ok(bytes) = std::fs::read(pack_dir.join(name)) {
            entries.push((name.to_string(), bytes));
        }
    }
    if let Some(index) = index_with_packweaver(pack_dir)? {
        entries.push(("index.json".into(), index));
        entries.push((PACK_JAR.into(), INSTALLER.to_vec()));
    }
    for file in filter.tree(pack_dir, "content") {
        entries.push((file.rel, std::fs::read(&file.path)?));
    }
    for file in filter.documents(pack_dir) {
        entries.push((file.rel, std::fs::read(&file.path)?));
    }
    for file in filter.tree(pack_dir, "overrides") {
        if only_overrides.is_some_and(|set| !set.contains(&file.rel)) {
            continue;
        }
        entries.push((file.rel, std::fs::read(&file.path)?));
    }
    entries.push(("modpack.url".into(), format!("{pack_url}\n").into_bytes()));
    if let Some(t) = dist_token.filter(|t| !t.is_empty()) {
        entries.push(("modpack.token".into(), format!("{t}\n").into_bytes()));
    }
    write_zip(&entries)
}

fn index_with_packweaver(pack_dir: &Path) -> Result<Option<Vec<u8>>> {
    let Some(mut files) = std::fs::read_to_string(pack_dir.join("index.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|index| index.get("files").and_then(Value::as_array).cloned())
    else {
        return Ok(None);
    };
    files.retain(|f| f.get("path").and_then(Value::as_str) != Some(PACK_JAR));
    files.push(json!({ "path": PACK_JAR, "sha1": hex::encode(Sha1::digest(INSTALLER)) }));
    files.sort_by(|a, b| {
        let path = |v: &Value| {
            v.get("path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        path(a).cmp(&path(b))
    });
    let text = serde_json::to_string_pretty(&json!({ "files": files }))?;
    Ok(Some(format!("{text}\n").into_bytes()))
}

fn write_zip(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut sorted: Vec<&(String, Vec<u8>)> = entries.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, bytes) in sorted {
            zip.start_file(name.clone(), options)?;
            zip.write_all(bytes)?;
        }
        zip.finish()?;
    }
    Ok(buf.into_inner())
}
