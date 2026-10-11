use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::content::{self, ContentItem};
use crate::instance;
use crate::lockfile::SourceFile;
use crate::manifest::{self, Manifest};
use crate::providers::ProviderId;
use crate::ptype::{self, ProjectType};
use ts_rs::TS;

pub const PACK_FILE: &str = "pack.toml";

const KNOWN_DATAPACK_FOLDERS: [&str; 3] = [
    ptype::DEFAULT_DATAPACKS,
    "config/paxi/datapacks",
    "config/openloader/data",
];
const LOADERS: [&str; 4] = ["neoforge", "forge", "fabric", "quilt"];

#[derive(Deserialize)]
struct PackToml {
    name: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    index: IndexRef,
    #[serde(default)]
    versions: BTreeMap<String, String>,
    #[serde(default)]
    options: PackOptions,
}

#[derive(Deserialize)]
struct IndexRef {
    #[serde(default = "default_index")]
    file: String,
}

impl Default for IndexRef {
    fn default() -> Self {
        IndexRef {
            file: default_index(),
        }
    }
}

fn default_index() -> String {
    "index.toml".into()
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct PackOptions {
    #[serde(default)]
    datapack_folder: Option<String>,
}

#[derive(Deserialize)]
struct IndexToml {
    #[serde(default)]
    files: Vec<IndexEntry>,
}

#[derive(Deserialize)]
struct IndexEntry {
    file: String,
    #[serde(default)]
    alias: Option<String>,
    #[serde(default)]
    metafile: bool,
    #[serde(default)]
    preserve: bool,
}

#[derive(Deserialize)]
struct MetaFile {
    name: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    side: Option<String>,
    #[serde(default)]
    pin: bool,
    #[serde(default)]
    download: Download,
    #[serde(default)]
    update: Update,
    #[serde(default)]
    option: Option<OptionInfo>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct Download {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    hash_format: Option<String>,
    #[serde(default)]
    hash: Option<String>,
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Deserialize, Default)]
struct Update {
    #[serde(default)]
    modrinth: Option<ModrinthUpdate>,
    #[serde(default)]
    curseforge: Option<CurseForgeUpdate>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ModrinthUpdate {
    mod_id: String,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct CurseForgeUpdate {
    project_id: u64,
    #[serde(default)]
    file_id: Option<u64>,
}

#[derive(Deserialize)]
struct OptionInfo {
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    default: bool,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct PackwizInfo {
    pub name: String,
    pub content: usize,
    pub files: usize,
    pub git: bool,
}

#[derive(Serialize, Default, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Converted {
    pub content: usize,
    pub files: usize,
    pub unlocked: usize,
    pub disabled: Vec<String>,
    pub skipped: Vec<String>,
    pub kept_packwiz: bool,
}

pub fn root_of(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        let named = path.file_name().is_some_and(|n| n == PACK_FILE);
        return named
            .then(|| path.parent().map(Path::to_path_buf))
            .flatten();
    }
    path.join(PACK_FILE).is_file().then(|| path.to_path_buf())
}

pub fn is_packwiz(dir: &Path) -> bool {
    dir.join(PACK_FILE).is_file() && !manifest::path_for(dir).is_file()
}

pub fn info(dir: &Path) -> Result<PackwizInfo> {
    let pack = read_pack(dir)?;
    let (_, index) = read_index(dir, &pack)?;
    let content = index.files.iter().filter(|f| f.metafile).count();
    Ok(PackwizInfo {
        name: pack.name,
        content,
        files: index.files.len() - content,
        git: crate::git::is_repo(dir),
    })
}

fn read_pack(dir: &Path) -> Result<PackToml> {
    let text = std::fs::read_to_string(dir.join(PACK_FILE))
        .with_context(|| format!("Could not read {PACK_FILE}"))?;
    toml::from_str(&text).map_err(|e| anyhow!("{PACK_FILE} isn't valid: {e}"))
}

fn read_index(dir: &Path, pack: &PackToml) -> Result<(PathBuf, IndexToml)> {
    let rel = crate::pathsafe::safe_zip_rel(&pack.index.file)
        .ok_or_else(|| anyhow!("{PACK_FILE} points to an index outside the pack"))?;
    let path = dir.join(rel);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("Could not read {}", pack.index.file))?;
    let index: IndexToml =
        toml::from_str(&text).map_err(|e| anyhow!("{} isn't valid: {e}", pack.index.file))?;
    let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
    Ok((base, index))
}

fn loader_of(versions: &BTreeMap<String, String>) -> (String, Option<String>) {
    for loader in LOADERS {
        if let Some(version) = versions.get(loader) {
            return (
                loader.to_string(),
                Some(version.clone()).filter(|v| !v.is_empty()),
            );
        }
    }
    ("vanilla".into(), None)
}

fn stem(rel: &str) -> String {
    let base = rel.rsplit('/').next().unwrap_or(rel);
    base.strip_suffix(".pw.toml")
        .or_else(|| base.strip_suffix(".toml"))
        .unwrap_or(base)
        .to_string()
}

fn folder_of(rel: &str) -> &str {
    rel.rsplit_once('/').map(|(folder, _)| folder).unwrap_or("")
}

fn type_of(folder: &str, datapacks: &Option<String>) -> Option<ProjectType> {
    match folder {
        "mods" => Some(ProjectType::Mod),
        "resourcepacks" => Some(ProjectType::Resourcepack),
        "shaderpacks" => Some(ProjectType::Shader),
        _ if datapacks.as_deref() == Some(folder) => Some(ProjectType::Datapack),
        _ => None,
    }
}

fn sides(side: Option<&str>) -> (&'static str, &'static str, &'static str) {
    match side {
        Some("client") => ("client", "required", "unsupported"),
        Some("server") => ("server", "unsupported", "required"),
        _ => ("auto", "required", "required"),
    }
}

fn hashes(download: &Download) -> (Option<String>, Option<String>) {
    let hash = download.hash.clone().filter(|h| !h.is_empty());
    match download.hash_format.as_deref() {
        Some("sha1") => (hash, None),
        Some("sha512") => (None, hash),
        _ => (None, None),
    }
}

fn item_from(meta: &MetaFile, slug: String, project_type: ProjectType) -> Option<ContentItem> {
    let (side, client_side, server_side) = sides(meta.side.as_deref());
    let mut sources = BTreeMap::new();
    if let Some(update) = &meta.update.modrinth {
        sources.insert(
            ProviderId::Modrinth,
            SourceFile {
                id: Some(update.mod_id.clone()),
                slug: slug.clone(),
                version_id: update.version.clone().unwrap_or_default(),
                ..Default::default()
            },
        );
    }
    if let Some(update) = &meta.update.curseforge {
        sources.insert(
            ProviderId::Curseforge,
            SourceFile {
                id: Some(update.project_id.to_string()),
                slug: slug.clone(),
                version_id: update.file_id.map(|f| f.to_string()).unwrap_or_default(),
                ..Default::default()
            },
        );
    }
    let from_curseforge = meta
        .download
        .mode
        .as_deref()
        .is_some_and(|m| m.contains("curseforge"));
    let preferred = match (
        sources.contains_key(&ProviderId::Modrinth),
        sources.contains_key(&ProviderId::Curseforge),
    ) {
        (true, true) if from_curseforge => ProviderId::Curseforge,
        (true, _) => ProviderId::Modrinth,
        (false, true) => ProviderId::Curseforge,
        (false, false) => {
            let url = meta.download.url.clone().filter(|u| !u.is_empty())?;
            let (sha1, sha512) = hashes(&meta.download);
            sources.insert(
                ProviderId::Url,
                SourceFile {
                    url: Some(url),
                    slug: slug.clone(),
                    filename: meta.filename.clone(),
                    sha1,
                    sha512,
                    ..Default::default()
                },
            );
            ProviderId::Url
        }
    };
    let off = meta
        .option
        .as_ref()
        .is_some_and(|o| o.optional && !o.default);
    Some(ContentItem {
        name: meta.name.clone(),
        project_type,
        file: String::new(),
        side: side.into(),
        explicit: true,
        disabled: off,
        pinned: meta.pin,
        preferred,
        sources,
        dependents: Vec::new(),
        optional_dependents: Vec::new(),
        no_bundle: false,
        client_side: client_side.into(),
        server_side: server_side.into(),
    })
}

fn download(url: &str, out: &Path) -> Result<()> {
    let response = reqwest::blocking::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()?
        .get(url)
        .send()?
        .error_for_status()?;
    let mut bytes = Vec::new();
    response.take(512 * 1024 * 1024).read_to_end(&mut bytes)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, bytes)?;
    Ok(())
}

fn place(from: &Path, to: &Path, moving: bool) -> Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if moving && std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    if moving {
        std::fs::remove_file(from)?;
    }
    Ok(())
}

fn remove_empty_dirs(root: &Path, start: &Path) {
    let mut dir = start.to_path_buf();
    while dir.starts_with(root) && dir != root {
        if std::fs::remove_dir(&dir).is_err() {
            break;
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break,
        }
    }
}

fn copy_docs(src: &Path, dest: &Path, skipped: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(src)?.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        let doc = lower.ends_with(".md") || lower.starts_with("license");
        if path.is_file() && doc && !dest.join(&name).exists() {
            if let Err(e) = std::fs::copy(&path, dest.join(&name)) {
                skipped.push(format!("{name}: couldn't copy it ({e})"));
            }
        }
    }
    Ok(())
}

pub fn convert(src: &Path, dest: &Path) -> Result<(Manifest, Converted)> {
    let in_place = crate::util::same_dir(src, dest);
    let moving = in_place && crate::git::is_repo(src);
    let pack = read_pack(src)?;
    let (base, index) = read_index(src, &pack)?;
    let minecraft = pack
        .versions
        .get("minecraft")
        .cloned()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("{PACK_FILE} has no Minecraft version"))?;
    let (loader, loader_version) = loader_of(&pack.versions);

    let mut datapacks = pack
        .options
        .datapack_folder
        .as_deref()
        .and_then(ptype::datapack_folder);
    if datapacks.is_none() {
        datapacks = index
            .files
            .iter()
            .filter(|f| f.metafile)
            .map(|f| folder_of(&f.file))
            .find(|folder| KNOWN_DATAPACK_FOLDERS.contains(folder))
            .map(str::to_string);
    }

    std::fs::create_dir_all(dest)?;
    let overrides = instance::overrides_dir(dest);
    let mut report = Converted {
        kept_packwiz: in_place && !moving,
        ..Default::default()
    };
    let mut unlocked: Vec<String> = Vec::new();
    let mut packwiz_files = vec![src.join(PACK_FILE), src.join(&pack.index.file)];
    let mut emptied: Vec<PathBuf> = Vec::new();

    for entry in &index.files {
        let Some(rel) = crate::pathsafe::safe_zip_rel(&entry.file) else {
            report
                .skipped
                .push(format!("{}: outside the pack", entry.file));
            continue;
        };
        let rel_text = rel.to_string_lossy().replace('\\', "/");
        let path = base.join(&rel);
        if entry.metafile {
            packwiz_files.push(path.clone());
            let meta: MetaFile = match std::fs::read_to_string(&path)
                .map_err(anyhow::Error::from)
                .and_then(|t| toml::from_str(&t).map_err(anyhow::Error::from))
            {
                Ok(meta) => meta,
                Err(e) => {
                    report.skipped.push(format!("{rel_text}: {e}"));
                    continue;
                }
            };
            let folder = folder_of(&rel_text);
            match type_of(folder, &datapacks) {
                Some(project_type) => match item_from(&meta, stem(&rel_text), project_type) {
                    Some(item) => {
                        if item.disabled {
                            report.disabled.push(item.name.clone());
                        }
                        content::add_item(dest, &item)?;
                        report.content += 1;
                    }
                    None => report.skipped.push(format!(
                        "{}: no Modrinth, CurseForge or download link",
                        meta.name
                    )),
                },
                None => match meta.download.url.as_deref().filter(|u| !u.is_empty()) {
                    Some(url) => {
                        let target = format!("{folder}/{}", meta.filename);
                        match crate::pathsafe::safe_zip_rel(&target) {
                            Some(target) => match download(url, &overrides.join(target)) {
                                Ok(()) => report.files += 1,
                                Err(e) => report.skipped.push(format!("{}: {e}", meta.name)),
                            },
                            None => report.skipped.push(format!("{}: bad file name", meta.name)),
                        }
                    }
                    None => report.skipped.push(format!(
                        "{}: in {folder}/, and only CurseForge knows where to download it",
                        meta.name
                    )),
                },
            }
            continue;
        }
        if !path.is_file() {
            report.skipped.push(format!("{rel_text}: missing"));
            continue;
        }
        let target = entry
            .alias
            .as_deref()
            .and_then(crate::pathsafe::safe_zip_rel)
            .unwrap_or_else(|| rel.clone());
        place(&path, &overrides.join(&target), moving)?;
        if moving {
            if let Some(parent) = path.parent() {
                emptied.push(parent.to_path_buf());
            }
        }
        report.files += 1;
        if entry.preserve {
            unlocked.push(target.to_string_lossy().replace('\\', "/"));
        }
    }

    let mf = Manifest {
        name: pack.name.clone(),
        version: pack
            .version
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "1.0.0".into()),
        minecraft,
        loader,
        loader_version,
        channel: manifest::Channel::Release,
        datapacks,
        packweave_md: true,
        format_version: 0,
    };
    manifest::write(dest, &mf)?;
    if !unlocked.is_empty() {
        crate::preserve::update(dest, &[], &unlocked)?;
        report.unlocked = unlocked.len();
    }
    if !in_place {
        copy_docs(src, dest, &mut report.skipped)?;
    }
    if moving {
        for file in packwiz_files.iter().chain([&src.join(".packwizignore")]) {
            if let Err(e) = crate::util::delete_file(file) {
                let name = file.strip_prefix(src).unwrap_or(file).display();
                report
                    .skipped
                    .push(format!("{name}: couldn't remove it ({e})"));
            }
            if let Some(parent) = file.parent() {
                emptied.push(parent.to_path_buf());
            }
        }
        emptied.sort();
        emptied.dedup();
        for dir in emptied.iter().rev() {
            remove_empty_dirs(src, dir);
        }
    }
    Ok((mf, report))
}
