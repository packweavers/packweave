use anyhow::{anyhow, Result};

use crate::ptype::{Folders, ProjectType};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rayon::prelude::*;
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use ts_rs::TS;

type HashKey = (PathBuf, u64, u64);
type HashEntry = (String, u32);
static HASH_CACHE: OnceLock<Mutex<HashMap<HashKey, HashEntry>>> = OnceLock::new();

fn hash_cache() -> &'static Mutex<HashMap<HashKey, HashEntry>> {
    HASH_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn clear_hash_cache() {
    if let Ok(mut cache) = hash_cache().lock() {
        cache.clear();
    }
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
    if let Some(v) = hash_cache().lock().ok().and_then(|c| c.get(&key).cloned()) {
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

const CONTENT_FOLDERS: &[&str] = &["/mods/", "/resourcepacks/", "/shaderpacks/", ".git/"];

pub const DEFAULT_GITIGNORE: &str = "\
.DS_Store

# packweave
.pack-local.json

# minecraft
overrides/saves/
overrides/backups/
overrides/logs/
overrides/crash-reports/
overrides/screenshots/
*.log
usercache.json
usernamecache.json
realms_persistence.json
command_history.txt
*.dat_old

# loader
.fabric/
.quilt/
overrides/natives/
.mixin.out/
.cache/
overrides/versions/
overrides/libraries/
overrides/assets/
overrides/debug/
overrides/downloads/
overrides/local/
overrides/cache/

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
# normalize line endings
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

pub fn ensure_gitignore(pack_dir: &Path) -> Result<()> {
    let gitignore = pack_dir.join(GITIGNORE_FILE);
    if !gitignore.exists() {
        crate::util::write_file(&gitignore, DEFAULT_GITIGNORE)?;
    }
    Ok(())
}

fn adopt_gitignore_defaults(pack_dir: &Path) -> Result<()> {
    let path = pack_dir.join(GITIGNORE_FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let have: std::collections::HashSet<&str> = text.lines().map(str::trim).collect();
    let missing: Vec<&str> = DEFAULT_GITIGNORE
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !have.contains(line))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let mut next = text.clone();
    if !next.ends_with('\n') {
        next.push('\n');
    }
    for line in missing {
        next.push_str(line);
        next.push('\n');
    }
    crate::util::write_file(&path, next)
}

pub fn create_defaults(pack_dir: &Path) -> Result<()> {
    let gitattributes = pack_dir.join(GITATTRIBUTES_FILE);
    if !gitattributes.exists() {
        crate::util::write_file(&gitattributes, DEFAULT_GITATTRIBUTES)?;
    }
    crate::preserve::ensure(pack_dir)
}

pub fn ensure_defaults(pack_dir: &Path) -> Result<()> {
    let Ok(text) = std::fs::read_to_string(crate::manifest::path_for(pack_dir)) else {
        return Ok(());
    };
    if crate::lenient::has_conflict(&text) {
        return Ok(());
    }
    let Ok(mut manifest) = crate::manifest::read(pack_dir) else {
        return Ok(());
    };
    if manifest.format_version >= crate::manifest::FORMAT_VERSION
        || !crate::preserve::migrate(pack_dir)?
    {
        return Ok(());
    }
    adopt_gitignore_defaults(pack_dir)?;
    crate::packignore::adopt_export_defaults(pack_dir)?;
    create_defaults(pack_dir)?;
    crate::content::retire_generated_readme(pack_dir)?;
    manifest.format_version = crate::manifest::FORMAT_VERSION;
    crate::manifest::write(pack_dir, &manifest)
        .map_err(|e| anyhow!("Couldn't write {}: {e}", crate::manifest::MANIFEST_FILE))
}

pub struct InstanceMod {
    pub filename: String,
    pub rel_path: String,
    pub project_type: ProjectType,
    pub sha1: String,
    pub murmur2: u32,
}

pub struct TrackedFile {
    pub path: String,
    pub sha1: String,
}

pub fn scan_content(instance: &Path, folders: &Folders) -> Vec<InstanceMod> {
    let mut candidates: Vec<(PathBuf, String, String, ProjectType)> = Vec::new();
    for project_type in ProjectType::ALL {
        let sub = folders.install_dir(project_type);
        let entries = match std::fs::read_dir(instance.join(sub)) {
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
            if !project_type.extensions().contains(&ext.as_str()) {
                continue;
            }
            let filename = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if project_type == ProjectType::Mod
                && filename.eq_ignore_ascii_case(crate::dist::INSTALLER_JAR)
            {
                continue;
            }
            candidates.push((p, format!("{sub}/{filename}"), filename, project_type));
        }
    }
    candidates
        .into_par_iter()
        .filter_map(|(path, rel_path, filename, project_type)| {
            let (sha1, murmur2) = cached_content_hash(&path)?;
            Some(InstanceMod {
                filename,
                rel_path,
                project_type,
                sha1,
                murmur2,
            })
        })
        .collect()
}

pub struct SyncIgnore {
    defaults: Gitignore,
    git: Gitignore,
}

impl SyncIgnore {
    pub fn load(pack_dir: &Path) -> Self {
        let mut builder = GitignoreBuilder::new(pack_dir);
        for pattern in CONTENT_FOLDERS {
            let _ = builder.add_line(None, pattern);
        }
        let datapacks = Folders::load(pack_dir).datapacks().to_string();
        for pattern in ["*.zip", "*.zip.disabled"] {
            let _ = builder.add_line(None, &format!("/{datapacks}/{pattern}"));
        }
        let _ = builder.add_line(None, &format!("/{}", crate::dist::INSTALLER_JAR));
        SyncIgnore {
            defaults: builder.build().unwrap_or_else(|_| Gitignore::empty()),
            git: crate::packignore::matcher(pack_dir, crate::packignore::IgnoreKind::Git),
        }
    }

    pub fn ignores(&self, rel: &str, is_dir: bool) -> bool {
        is_ignored(&self.defaults, rel, is_dir) || self.ignores_user(rel, is_dir)
    }

    pub fn ignores_user(&self, rel: &str, is_dir: bool) -> bool {
        is_ignored(&self.git, rel, is_dir)
            || is_ignored(&self.git, &format!("{OVERRIDES_DIR}/{rel}"), is_dir)
    }
}

fn is_ignored(matcher: &Gitignore, rel: &str, is_dir: bool) -> bool {
    matcher
        .matched_path_or_any_parents(Path::new(rel), is_dir)
        .is_ignore()
}

pub fn scan_tracked(
    instance: &Path,
    matcher: &SyncIgnore,
    attrs: &GitAttributes,
) -> Vec<TrackedFile> {
    let mut files = Vec::new();
    collect_files(instance, instance, matcher, &mut files);
    hash_all(files, attrs)
}

pub fn overrides_dir(pack_dir: &Path) -> PathBuf {
    pack_dir.join(OVERRIDES_DIR)
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct LocalContent {
    pub rel_path: String,
    pub filename: String,
    pub project_type: ProjectType,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub meta: Option<crate::modmeta::ModMeta>,
}

pub fn list_unpublished(pack_dir: &Path) -> Vec<LocalContent> {
    let base = overrides_dir(pack_dir);
    let folders = Folders::load(pack_dir);
    let mut candidates: Vec<(PathBuf, String, String, u64, ProjectType)> = Vec::new();
    for project_type in ProjectType::ALL {
        let dir = folders.install_dir(project_type);
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
            let lower = filename.to_lowercase();
            if project_type == ProjectType::Datapack
                && !(lower.ends_with(".zip") || lower.ends_with(".zip.disabled"))
            {
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
    matcher: &SyncIgnore,
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

fn sha1_normalized(path: &Path, rel: &str, attrs: &GitAttributes) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let binary = attrs.is_binary(rel).unwrap_or_else(|| looks_binary(&bytes));
    let data = if binary { bytes } else { normalize_eol(&bytes) };
    let mut hasher = Sha1::new();
    hasher.update(&data);
    Some(hex::encode(hasher.finalize()))
}

fn hash_all(files: Vec<(PathBuf, String)>, attrs: &GitAttributes) -> Vec<TrackedFile> {
    files
        .into_par_iter()
        .filter_map(|(path, rel)| {
            sha1_normalized(&path, &rel, attrs).map(|sha1| TrackedFile { path: rel, sha1 })
        })
        .collect()
}

fn collect_files(dir: &Path, base: &Path, matcher: &SyncIgnore, out: &mut Vec<(PathBuf, String)>) {
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
            if !matcher.ignores(&rel, true) {
                collect_files(&p, base, matcher, out);
            }
        } else if p.is_file() && !matcher.ignores(&rel, false) {
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

fn file_in_use(e: &std::io::Error) -> bool {
    cfg!(windows) && matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33))
}

fn copy_checked(src: &Path, dst: &Path, rel: &str) -> Result<()> {
    match std::fs::copy(src, dst) {
        Ok(_) => Ok(()),
        Err(e) if file_in_use(&e) => Err(anyhow!(
            "{rel} is in use, close Minecraft and your launcher, then try again"
        )),
        Err(e) => Err(anyhow!("couldn't copy {rel}: {e}")),
    }
}

pub fn copy_into_overrides(pack_dir: &Path, instance: &Path, rel: &str) -> Result<()> {
    safe_rel(rel)?;
    let src = instance.join(rel);
    let dst = overrides_dir(pack_dir).join(rel);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    copy_checked(&src, &dst, rel)
}

pub fn copy_into_instance(pack_dir: &Path, instance: &Path, rel: &str) -> Result<()> {
    safe_rel(rel)?;
    let src = overrides_dir(pack_dir).join(rel);
    let dst = instance.join(rel);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    copy_checked(&src, &dst, rel)
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
        if let Err(e) = std::fs::remove_file(p) {
            if file_in_use(&e) {
                return Err(anyhow!(
                    "{rel} is in use, close Minecraft and your launcher, \
                     then try again"
                ));
            }
            return Err(anyhow!("couldn't remove {rel}: {e}"));
        }
    }
    Ok(())
}
