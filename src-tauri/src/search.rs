use anyhow::{anyhow, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;

const MAX_CONTENT_SIZE: u64 = 1_000_000;
const MAX_RESULTS: usize = 2000;
const MAX_PER_FILE: usize = 50;
const SKIP_DIRS: &[&str] = &[
    "content",
    "mods",
    "resourcepacks",
    "shaderpacks",
    "datapacks",
    "saves",
    "backups",
    "logs",
    "crash-reports",
    ".git",
    "versions",
    "libraries",
    "assets",
    "natives",
    "node_modules",
    "target",
];

#[derive(Deserialize, Default, Clone, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "types.ts")]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
    pub include: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct FileMatch {
    pub path: String,
    pub line: u32,
    pub text: String,
    pub ranges: Vec<(u32, u32)>,
}

fn pattern(query: &str, options: &SearchOptions) -> Result<Regex> {
    let mut source = if options.regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    if options.whole_word {
        source = format!(r"\b(?:{source})\b");
    }
    RegexBuilder::new(&source)
        .case_insensitive(!options.case_sensitive)
        .build()
        .map_err(|e| anyhow!("That pattern isn't valid: {e}"))
}

fn include_set(include: &str) -> Option<GlobSet> {
    let globs: Vec<&str> = include
        .split(',')
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .collect();
    if globs.is_empty() {
        return None;
    }
    let mut builder = GlobSetBuilder::new();
    for g in globs {
        let patterns = if g.ends_with('/') {
            vec![format!("{g}**")]
        } else if g.contains('/') || g.contains('*') {
            vec![g.to_string()]
        } else {
            vec![format!("**/{g}"), format!("**/{g}/**")]
        };
        for pattern in patterns {
            if let Ok(glob) = Glob::new(&pattern) {
                builder.add(glob);
            }
        }
    }
    builder.build().ok()
}

fn files_under(root: &Path, include: Option<&GlobSet>) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    collect(root, &mut files);
    files
        .into_iter()
        .filter_map(|p| {
            let rel = p
                .strip_prefix(root)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            let game = rel.strip_prefix("overrides/").unwrap_or(&rel);
            if include.is_some_and(|set| !set.is_match(&rel) && !set.is_match(game)) {
                return None;
            }
            Some((p, rel))
        })
        .collect()
}

pub fn index_files(root: &str) -> Vec<String> {
    let root = PathBuf::from(root);
    let mut out: Vec<String> = files_under(&root, None)
        .into_iter()
        .map(|(_, rel)| rel)
        .take(20_000)
        .collect();
    out.sort();
    out
}

pub fn search_files(root: &str, query: &str, options: &SearchOptions) -> Result<Vec<FileMatch>> {
    if query.is_empty() {
        return Ok(vec![]);
    }
    let re = pattern(query, options)?;
    let root = PathBuf::from(root);
    let include = include_set(&options.include);
    let mut results: Vec<FileMatch> = files_under(&root, include.as_ref())
        .par_iter()
        .flat_map(|(path, rel)| search_one(path, rel, &re))
        .collect();
    results.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    results.truncate(MAX_RESULTS);
    Ok(results)
}

pub fn replace_in_files(
    root: &str,
    query: &str,
    options: &SearchOptions,
    replacement: &str,
    only: &[String],
) -> Result<u32> {
    let re = pattern(query, options)?;
    let root = PathBuf::from(root);
    let include = include_set(&options.include);
    let mut count = 0;
    for (path, rel) in files_under(&root, include.as_ref()) {
        if !only.is_empty() && !only.contains(&rel) {
            continue;
        }
        let Some(text) = read_text(&path) else {
            continue;
        };
        let hits = re.find_iter(&text).count() as u32;
        if hits == 0 {
            continue;
        }
        let replaced = if options.regex {
            re.replace_all(&text, replacement)
        } else {
            re.replace_all(&text, regex::NoExpand(replacement))
        };
        std::fs::write(&path, replaced.as_bytes())?;
        count += hits;
    }
    Ok(count)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            collect(&p, out);
        } else if p.is_file() {
            out.push(p);
        }
    }
}

fn read_text(path: &Path) -> Option<String> {
    let size = path.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
    if size > MAX_CONTENT_SIZE {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    if bytes.iter().take(8000).any(|b| *b == 0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn search_one(path: &Path, rel: &str, re: &Regex) -> Vec<FileMatch> {
    let mut out = Vec::new();
    let Some(text) = read_text(path) else {
        return out;
    };
    for (i, line) in text.lines().enumerate() {
        let ranges: Vec<(u32, u32)> = re
            .find_iter(line)
            .map(|m| {
                let start = line[..m.start()].chars().count() as u32;
                let len = m.as_str().chars().count() as u32;
                (start, start + len)
            })
            .collect();
        if ranges.is_empty() {
            continue;
        }
        out.push(FileMatch {
            path: rel.to_string(),
            line: (i + 1) as u32,
            text: line.chars().take(400).collect(),
            ranges,
        });
        if out.len() >= MAX_PER_FILE {
            break;
        }
    }
    out
}
