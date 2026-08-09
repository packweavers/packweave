use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};

const MAX_CONTENT_SIZE: u64 = 1_000_000;
const MAX_RESULTS: usize = 400;
const MAX_PER_FILE: usize = 6;
const SKIP_DIRS: &[&str] = &[
    "mods",
    "resourcepacks",
    "shaderpacks",
    "saves",
    "backups",
    "logs",
    "crash-reports",
    ".git",
    "versions",
    "libraries",
    "assets",
    "node_modules",
    "target",
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMatch {
    pub path: String,
    pub line: u32,
    pub text: String,
}

pub fn search_files(root: &str, query: &str) -> Vec<FileMatch> {
    let query = query.trim();
    if query.len() < 2 {
        return vec![];
    }
    let root_path = PathBuf::from(root);
    let mut files: Vec<PathBuf> = Vec::new();
    collect(&root_path, &mut files);
    let needle = query.to_lowercase();

    let mut results: Vec<FileMatch> = files
        .par_iter()
        .flat_map(|p| search_one(p, &root_path, &needle))
        .collect();

    results.sort_by(|a, b| {
        let an = a.line == 0;
        let bn = b.line == 0;
        bn.cmp(&an)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.line.cmp(&b.line))
    });
    results.truncate(MAX_RESULTS);
    results
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

fn search_one(path: &Path, root: &Path, needle: &str) -> Vec<FileMatch> {
    let rel = path
        .strip_prefix(root)
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let mut out = Vec::new();

    if rel.to_lowercase().contains(needle) {
        out.push(FileMatch {
            path: rel.clone(),
            line: 0,
            text: String::new(),
        });
    }

    let size = path.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
    if size <= MAX_CONTENT_SIZE {
        if let Some(text) = std::fs::read(path)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
        {
            let mut hits = 0;
            for (i, line) in text.lines().enumerate() {
                if line.to_lowercase().contains(needle) {
                    out.push(FileMatch {
                        path: rel.clone(),
                        line: (i + 1) as u32,
                        text: line.trim().chars().take(200).collect(),
                    });
                    hits += 1;
                    if hits >= MAX_PER_FILE {
                        break;
                    }
                }
            }
        }
    }
    out
}
