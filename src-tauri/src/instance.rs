use anyhow::{anyhow, Result};

use crate::ptype::ProjectType;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rayon::prelude::*;
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

type HashKey = (PathBuf, u64, u64);
type HashEntry = (String, u32);
static HASH_CACHE: OnceLock<Mutex<HashMap<HashKey, HashEntry>>> =
    OnceLock::new();

fn hash_cache() -> &'static Mutex<HashMap<HashKey, HashEntry>> {
    HASH_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_content_hash(path: &Path) -> Option<HashEntry> {
    let meta = std::fs::metadata(path).ok()?;
    let size = meta.len();
    let mtime = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let key = (path.to_path_buf(), mtime, size);
    if let Some(v) = hash_cache().lock().ok().and_then(|c| c.get(&key).cloned())
    {
        return Some(v);
    }
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    let entry: HashEntry = (
        hex::encode(hasher.finalize()),
        crate::curseforge::cf_fingerprint(&bytes),
    );
    if let Ok(mut c) = hash_cache().lock() {
        c.insert(key, entry.clone());
    }
    Some(entry)
}

pub const OVERRIDES_DIR: &str = "overrides";
pub const GITIGNORE_FILE: &str = ".gitignore";

const DEFAULT_IGNORES: &[&str] = &[
    "mods/",
    "resourcepacks/",
    "shaderpacks/",
    ".git/",
    "versions/",
    "libraries/",
    "assets/",
    "natives/",
    "downloads/",
    "local/",
    "debug/",
    ".cache/",
    ".fabric/",
    ".quilt/",
    ".mixin.out/",
    "saves/",
    "backups/",
    "logs/",
    "crash-reports/",
    "screenshots/",
    "*.log",
    "*.dat_old",
    "usercache.json",
    "usernamecache.json",
    "realms_persistence.json",
    "servers.dat",
    "command_history.txt",
    "*.bak",
];

pub const DEFAULT_GITIGNORE: &str = "\
.DS_Store

# packweave
.pack-local.json

# minecraft
saves/
backups/
logs/
crash-reports/
screenshots/
*.log
usercache.json
usernamecache.json
realms_persistence.json
command_history.txt
servers.dat
*.dat_old

# loader
.fabric/
.quilt/
natives/
.mixin.out/
.cache/
versions/
libraries/
assets/
debug/
downloads/
local/
cache/

# mods
*.bak
/emi.json
ponders_watched.json
dynamic-data-pack-cache/
dynamic-resource-pack-cache/
fieldguide_cache/
fancymenu_data/
Distant_Horizons_server_data/
emojiful/
figura/
server-resource-packs/
schematics/
.sable/
imgui.ini
moddata/
mods/.connector/
mods/.index/
coremods/
config/spark/tmp-client/
cherishedworlds-favorites.dat
.probe/
.vscode/probe.code-snippets
config/sodium-fingerprint.json
";

pub const GITATTRIBUTES_FILE: &str = ".gitattributes";

pub const DEFAULT_GITATTRIBUTES: &str = "\
# Normalize line endings
* text=auto eol=lf

# Binaries
*.jar binary
*.zip binary
*.mrpack binary
*.png binary
*.jpg binary
*.jpeg binary
*.gif binary
*.ico binary
*.ogg binary
*.nbt binary
*.dat binary
";

pub fn ensure_defaults(pack_dir: &Path) {
    let gitignore = pack_dir.join(GITIGNORE_FILE);
    if !gitignore.exists() {
        let _ = std::fs::write(&gitignore, DEFAULT_GITIGNORE);
    }
    let gitattributes = pack_dir.join(GITATTRIBUTES_FILE);
    if !gitattributes.exists() {
        let _ = std::fs::write(&gitattributes, DEFAULT_GITATTRIBUTES);
    }
}

pub fn is_os_junk(rel: &str) -> bool {
    rel.split('/').map(|c| c.trim()).any(|c| {
        c.starts_with("._")
            || c.starts_with("Thumbs.db")
            || matches!(
                c,
                ".DS_Store"
                    | ".AppleDouble"
                    | ".LSOverride"
                    | ".Spotlight-V100"
                    | ".Trashes"
                    | ".fseventsd"
                    | ".DocumentRevisions-V100"
                    | ".TemporaryItems"
                    | ".apdisk"
                    | "ehthumbs.db"
                    | "ehthumbs_vista.db"
                    | "desktop.ini"
                    | ".directory"
                    | "$RECYCLE.BIN"
                    | "System Volume Information"
            )
    })
}

pub struct InstanceMod {
    pub filename: String,
    pub rel_path: String,
    pub sha1: String,
    pub murmur2: u32,
}

pub struct TrackedFile {
    pub path: String,
    pub sha1: String,
}

pub fn scan_content(instance: &Path) -> Vec<InstanceMod> {
    let mut candidates: Vec<(PathBuf, String, String)> = Vec::new();
    for sub in ["mods", "resourcepacks", "shaderpacks"] {
        let dir = instance.join(sub);
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let ext = p
                .extension()
                .map(|x| x.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if ext != "jar" && ext != "zip" {
                continue;
            }
            let filename = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            candidates.push((p, format!("{sub}/{filename}"), filename));
        }
    }
    candidates
        .into_par_iter()
        .filter_map(|(path, rel_path, filename)| {
            let (sha1, murmur2) = cached_content_hash(&path)?;
            Some(InstanceMod {
                filename,
                rel_path,
                sha1,
                murmur2,
            })
        })
        .collect()
}

pub fn ignore_matcher(pack_dir: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(pack_dir);
    let gitignore = pack_dir.join(GITIGNORE_FILE);
    if gitignore.exists() {
        let _ = builder.add(&gitignore);
    }
    for pattern in DEFAULT_IGNORES {
        let _ = builder.add_line(None, pattern);
    }
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

pub fn user_ignore_matcher(pack_dir: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(pack_dir);
    let gitignore = pack_dir.join(GITIGNORE_FILE);
    if gitignore.exists() {
        let _ = builder.add(&gitignore);
    }
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

pub fn is_ignored(matcher: &Gitignore, rel: &str, is_dir: bool) -> bool {
    matcher
        .matched_path_or_any_parents(Path::new(rel), is_dir)
        .is_ignore()
}

pub fn add_ignore(pack_dir: &Path, pattern: &str) -> Result<()> {
    let path = pack_dir.join(GITIGNORE_FILE);
    let mut content = std::fs::read_to_string(&path).unwrap_or_default();
    if content.lines().any(|l| l.trim() == pattern) {
        return Ok(());
    }
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(pattern);
    content.push('\n');
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn read_gitignore(pack_dir: &Path) -> String {
    std::fs::read_to_string(pack_dir.join(GITIGNORE_FILE)).unwrap_or_default()
}

pub fn write_gitignore(pack_dir: &Path, content: &str) -> Result<()> {
    let normalized = if content.ends_with('\n') || content.is_empty() {
        content.to_string()
    } else {
        format!("{content}\n")
    };
    std::fs::write(pack_dir.join(GITIGNORE_FILE), normalized)?;
    Ok(())
}

pub fn scan_tracked(
    instance: &Path,
    matcher: &Gitignore,
    attrs: &GitAttributes,
) -> Vec<TrackedFile> {
    let mut files = Vec::new();
    collect_files(instance, instance, matcher, &mut files);
    hash_all(files, attrs)
}

pub fn overrides_dir(pack_dir: &Path) -> PathBuf {
    pack_dir.join(OVERRIDES_DIR)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalContent {
    pub rel_path: String,
    pub filename: String,
    pub project_type: ProjectType,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<crate::modmeta::ModMeta>,
}

pub fn list_unpublished(pack_dir: &Path) -> Vec<LocalContent> {
    let base = overrides_dir(pack_dir);
    let dirs = [
        ("mods", ProjectType::Mod),
        ("resourcepacks", ProjectType::Resourcepack),
        ("shaderpacks", ProjectType::Shader),
    ];
    let mut candidates: Vec<(PathBuf, String, String, u64, ProjectType)> =
        Vec::new();
    for (dir, project_type) in dirs {
        let entries = match std::fs::read_dir(base.join(dir)) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if filename.starts_with('.') {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            candidates.push((
                path,
                format!("{dir}/{filename}"),
                filename,
                size,
                project_type,
            ));
        }
    }
    let mut out: Vec<LocalContent> = candidates
        .into_par_iter()
        .map(
            |(path, rel_path, filename, size, project_type)| LocalContent {
                meta: crate::modmeta::read_archive(&path),
                rel_path,
                filename,
                project_type,
                size,
            },
        )
        .collect();
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    out
}

pub fn scan_overrides(
    pack_dir: &Path,
    matcher: &Gitignore,
    attrs: &GitAttributes,
) -> Vec<TrackedFile> {
    let base = overrides_dir(pack_dir);
    let mut files = Vec::new();
    if base.is_dir() {
        collect_files(&base, &base, matcher, &mut files);
    }
    hash_all(files, attrs)
}

pub struct GitAttributes {
    rules: Vec<(String, Option<bool>)>,
}

impl GitAttributes {
    pub fn load(pack_dir: &Path) -> Self {
        let text = std::fs::read_to_string(pack_dir.join(GITATTRIBUTES_FILE))
            .unwrap_or_else(|_| DEFAULT_GITATTRIBUTES.to_string());
        let mut rules = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let pattern = match parts.next() {
                Some(p) => p.to_string(),
                None => continue,
            };
            let mut binary: Option<bool> = None;
            for attr in parts {
                match attr {
                    "binary" | "-text" => binary = Some(true),
                    "text" => binary = Some(false),
                    "text=auto" => binary = None,
                    a if a.starts_with("text=") => binary = Some(false),
                    _ => {}
                }
            }
            rules.push((pattern, binary));
        }
        GitAttributes { rules }
    }

    fn is_binary(&self, rel: &str) -> Option<bool> {
        let mut result = None;
        for (pattern, binary) in &self.rules {
            if attr_matches(pattern, rel) {
                result = *binary;
            }
        }
        result
    }
}

fn attr_matches(pattern: &str, rel: &str) -> bool {
    let pattern = pattern.trim_start_matches('/');
    if pattern == "*" || pattern == "**" {
        return true;
    }
    let base = rel.rsplit('/').next().unwrap_or(rel);
    if let Some(ext) = pattern.strip_prefix("*.") {
        return base.ends_with(&format!(".{ext}"));
    }
    pattern == rel || pattern == base || rel.ends_with(&format!("/{pattern}"))
}

pub fn normalize_eol(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            out.push(b'\n');
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                i += 1;
            }
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    out
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|&b| b == 0)
}

fn sha1_normalized(
    path: &Path,
    rel: &str,
    attrs: &GitAttributes,
) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let binary = attrs.is_binary(rel).unwrap_or_else(|| looks_binary(&bytes));
    let data = if binary { bytes } else { normalize_eol(&bytes) };
    let mut hasher = Sha1::new();
    hasher.update(&data);
    Some(hex::encode(hasher.finalize()))
}

fn hash_all(
    files: Vec<(PathBuf, String)>,
    attrs: &GitAttributes,
) -> Vec<TrackedFile> {
    files
        .into_par_iter()
        .filter_map(|(path, rel)| {
            sha1_normalized(&path, &rel, attrs)
                .map(|sha1| TrackedFile { path: rel, sha1 })
        })
        .collect()
}

fn collect_files(
    dir: &Path,
    base: &Path,
    matcher: &Gitignore,
    out: &mut Vec<(PathBuf, String)>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let rel = match p.strip_prefix(base) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if p.is_dir() {
            if !is_ignored(matcher, &rel, true) {
                collect_files(&p, base, matcher, out);
            }
        } else if p.is_file() && !is_ignored(matcher, &rel, false) {
            out.push((p, rel));
        }
    }
}

fn safe_rel(rel: &str) -> Result<()> {
    if rel.split(['/', '\\']).any(|c| c == "..") {
        return Err(anyhow!("unsafe path: {rel}"));
    }
    Ok(())
}

pub fn copy_into_overrides(
    pack_dir: &Path,
    instance: &Path,
    rel: &str,
) -> Result<()> {
    safe_rel(rel)?;
    let src = instance.join(rel);
    let dst = overrides_dir(pack_dir).join(rel);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&src, &dst)?;
    Ok(())
}

pub fn copy_into_instance(
    pack_dir: &Path,
    instance: &Path,
    rel: &str,
) -> Result<()> {
    safe_rel(rel)?;
    let src = overrides_dir(pack_dir).join(rel);
    let dst = instance.join(rel);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&src, &dst)?;
    Ok(())
}

pub fn remove_from_overrides(pack_dir: &Path, rel: &str) -> Result<()> {
    safe_rel(rel)?;
    let p = overrides_dir(pack_dir).join(rel);
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

pub fn remove_from_instance(instance: &Path, rel: &str) -> Result<()> {
    safe_rel(rel)?;
    let p = instance.join(rel);
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eol_normalizes_crlf_and_cr() {
        assert_eq!(normalize_eol(b"a\r\nb\rc\n"), b"a\nb\nc\n");
        assert_eq!(normalize_eol(b"plain"), b"plain");
    }

    #[test]
    fn default_attributes_mark_jars_binary_configs_text() {
        let attrs = GitAttributes {
            rules: vec![
                ("*".into(), None),
                ("*.jar".into(), Some(true)),
                ("*.png".into(), Some(true)),
            ],
        };
        assert_eq!(attrs.is_binary("mods/sodium.jar"), Some(true));
        assert_eq!(attrs.is_binary("config/foo.toml"), None);
    }

    #[test]
    fn attr_matches_patterns() {
        assert!(attr_matches("*", "config/foo.toml"));
        assert!(attr_matches("*.jar", "mods/a.jar"));
        assert!(!attr_matches("*.jar", "mods/a.json"));
        assert!(attr_matches("config/options.txt", "config/options.txt"));
    }
}
