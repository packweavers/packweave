use anyhow::Result;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::git;
use crate::instance;

pub fn build_snapshot(
    pack_dir: &Path,
    dist_token: Option<&str>,
) -> Result<Vec<u8>> {
    build(pack_dir, None, dist_token)
}

pub fn build_delta(
    pack_dir: &Path,
    prev_tag: &str,
    dist_token: Option<&str>,
) -> Option<Vec<u8>> {
    let changed: HashSet<String> =
        git::changed_paths_since(pack_dir, prev_tag, "overrides")
            .ok()?
            .into_iter()
            .collect();
    build(pack_dir, Some(&changed), dist_token).ok()
}

fn build(
    pack_dir: &Path,
    only_overrides: Option<&HashSet<String>>,
    dist_token: Option<&str>,
) -> Result<Vec<u8>> {
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for name in ["modpack.json", "index.json", "preserve.txt"] {
        if let Ok(bytes) = std::fs::read(pack_dir.join(name)) {
            entries.push((name.to_string(), bytes));
        }
    }
    collect_tree(pack_dir, "content", None, &mut entries)?;
    collect_tree(pack_dir, "overrides", only_overrides, &mut entries)?;
    if let Some(t) = dist_token.filter(|t| !t.is_empty()) {
        entries.push(("modpack.token".into(), format!("{t}\n").into_bytes()));
    }
    write_zip(&entries)
}

fn collect_tree(
    pack_dir: &Path,
    sub: &str,
    only: Option<&HashSet<String>>,
    entries: &mut Vec<(String, Vec<u8>)>,
) -> Result<()> {
    let base = pack_dir.join(sub);
    if !base.is_dir() {
        return Ok(());
    }
    let mut stack = vec![base.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)?.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                let rel = match p.strip_prefix(pack_dir) {
                    Ok(r) => r.to_string_lossy().replace('\\', "/"),
                    Err(_) => continue,
                };
                let inner = rel
                    .strip_prefix(&format!("{sub}/"))
                    .unwrap_or(&rel)
                    .to_string();
                if instance::is_os_junk(&inner) {
                    continue;
                }
                if only.map(|set| !set.contains(&rel)).unwrap_or(false) {
                    continue;
                }
                entries.push((rel, std::fs::read(&p)?));
            }
        }
    }
    Ok(())
}

fn write_zip(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);
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
