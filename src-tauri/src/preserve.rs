use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::path::Path;

pub const FILE: &str = ".packweaveunlock";

const OLD_FILE: &str = "preserve.txt";

pub const DEFAULTS: &str = "\
# Player edits to files matching these patterns will persist with updates.
# One pattern per line, relative to .minecraft.
# * matches within a folder, ** matches any depth.
# file#setting matches a single setting in a config file.
# ! negates a pattern.
#
# config/somemod.toml
# config/somemod/**
# config/somemod.toml#client.fov
# !config/somemod.toml#client.debug
options.txt
servers.dat
";

const OLD_TEMPLATE: &str = "\
# Player edits to files matching these patterns will persist with updates.
# One pattern per line, relative to .minecraft.
# * matches within a folder, ** matches any depth.
#
# config/somemod.toml
# config/somemod/**
";

fn clean_path(raw: &str) -> Option<String> {
    let folder = raw.ends_with('/');
    let mut parts = Vec::new();
    for part in raw.split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return None;
        }
        parts.push(part);
    }
    if parts.is_empty() {
        return None;
    }
    let mut path = parts.join("/");
    if folder {
        path.push('/');
    }
    Some(path)
}

pub fn canonical(line: &str) -> Option<String> {
    let mut text = line.trim_start_matches('\u{feff}').trim();
    if text.is_empty() || text.starts_with('#') {
        return None;
    }
    let lock = text.starts_with('!');
    if lock {
        text = text[1..].trim();
    }
    let text = text.replace('\\', "/");
    let (path, key) = match text.split_once('#') {
        Some((path, key)) => (path, Some(key.trim()).filter(|k| !k.is_empty())),
        None => (text.as_str(), None),
    };
    let path = clean_path(path.trim())?;
    let mut out = String::new();
    if lock {
        out.push('!');
    }
    out.push_str(&path);
    if let Some(key) = key {
        out.push('#');
        out.push_str(key);
    }
    Some(out)
}

pub fn patterns(pack_dir: &Path) -> Vec<String> {
    std::fs::read_to_string(pack_dir.join(FILE))
        .unwrap_or_default()
        .lines()
        .filter_map(canonical)
        .collect()
}

pub fn update(pack_dir: &Path, remove: &[String], add: &[String]) -> Result<Vec<String>> {
    let path = pack_dir.join(FILE);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| DEFAULTS.to_string());
    let added: Vec<String> = add.iter().filter_map(|r| canonical(r)).collect();
    let gone: HashSet<String> = remove
        .iter()
        .filter_map(|r| canonical(r))
        .chain(added.iter().cloned())
        .collect();
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| canonical(line).is_none_or(|rule| !gone.contains(&rule)))
        .map(str::to_string)
        .collect();
    for rule in added {
        if !lines
            .iter()
            .any(|l| canonical(l).as_deref() == Some(rule.as_str()))
        {
            lines.push(rule);
        }
    }
    std::fs::write(&path, format!("{}\n", lines.join("\n")))?;
    Ok(patterns(pack_dir))
}

pub fn reset(pack_dir: &Path) -> Result<Vec<String>> {
    std::fs::write(pack_dir.join(FILE), DEFAULTS)?;
    Ok(patterns(pack_dir))
}

pub fn ensure(pack_dir: &Path) -> Result<()> {
    let path = pack_dir.join(FILE);
    if !path.exists() {
        return crate::util::write_file(&path, DEFAULTS);
    }
    let untouched = std::fs::read_to_string(&path)
        .is_ok_and(|text| text.replace("\r\n", "\n").trim() == OLD_TEMPLATE.trim());
    if untouched {
        crate::util::write_file(&path, DEFAULTS)?;
    }
    Ok(())
}

pub fn migrate(pack_dir: &Path) -> Result<bool> {
    let old = pack_dir.join(OLD_FILE);
    let new = pack_dir.join(FILE);
    if new.exists() || !old.is_file() {
        return Ok(true);
    }
    let bytes = std::fs::read(&old).unwrap_or_default();
    if crate::lenient::has_conflict(&String::from_utf8_lossy(&bytes)) {
        return Ok(false);
    }
    std::fs::rename(old, new).map_err(|e| anyhow!("Couldn't rename {OLD_FILE} to {FILE}: {e}"))?;
    Ok(true)
}
