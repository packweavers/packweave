use anyhow::{anyhow, Result};
use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use crate::content::{self, ContentItem};
use crate::convert::{copy_docs, place, remove_empty_dirs, ConvertInfo, Converted};
use crate::instance;
use crate::lockfile::SourceFile;
use crate::manifest::{self, Manifest};
use crate::packignore::{self, IgnoreKind};
use crate::providers::ProviderId;
use crate::ptype::{self, ProjectType};

pub const CONFIG_FILE: &str = "pakku.json";
pub const LOCK_FILE: &str = "pakku-lock.json";
const MANUAL_DIR: &str = ".pakku";
const MANUAL_OVERRIDES: [(&str, &str); 3] = [
    ("overrides", "both"),
    ("server-overrides", "server"),
    ("client-overrides", "client"),
];

#[derive(Deserialize, Default)]
struct LockFile {
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    mc_versions: Vec<String>,
    #[serde(default)]
    loaders: BTreeMap<String, String>,
    #[serde(default)]
    projects: Vec<Project>,
}

#[derive(Deserialize)]
struct Project {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    slug: BTreeMap<String, String>,
    #[serde(default)]
    name: BTreeMap<String, String>,
    #[serde(default)]
    id: BTreeMap<String, String>,
    #[serde(default)]
    update_strategy: Option<String>,
    #[serde(default)]
    redistributable: Option<bool>,
    #[serde(default)]
    aliases: Option<Vec<String>>,
    #[serde(default)]
    export: Option<bool>,
    #[serde(default)]
    files: Vec<ProjectFile>,
}

#[derive(Deserialize)]
struct ProjectFile {
    #[serde(default, rename = "type")]
    platform: String,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    id: String,
    #[serde(default)]
    parent_id: String,
    #[serde(default)]
    hashes: Option<BTreeMap<String, String>>,
}

#[derive(Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    overrides: Vec<String>,
    #[serde(default)]
    server_overrides: Vec<String>,
    #[serde(default)]
    client_overrides: Vec<String>,
    #[serde(default)]
    paths: BTreeMap<String, String>,
    #[serde(default)]
    projects: BTreeMap<String, ProjectConfig>,
}

#[derive(Deserialize, Default)]
struct ProjectConfig {
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    side: Option<String>,
    #[serde(default)]
    update_strategy: Option<String>,
    #[serde(default)]
    redistributable: Option<bool>,
    #[serde(default)]
    export: Option<bool>,
}

struct Rule {
    glob: GlobMatcher,
    keep: bool,
}

pub fn root_of(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        let named = path
            .file_name()
            .is_some_and(|n| n == CONFIG_FILE || n == LOCK_FILE);
        return named
            .then(|| path.parent().map(Path::to_path_buf))
            .flatten();
    }
    path.join(LOCK_FILE).is_file().then(|| path.to_path_buf())
}

pub fn is_pakku(dir: &Path) -> bool {
    dir.join(LOCK_FILE).is_file() && !manifest::path_for(dir).is_file()
}

fn read_lock(dir: &Path) -> Result<LockFile> {
    let text = std::fs::read_to_string(dir.join(LOCK_FILE))
        .map_err(|e| anyhow!("Couldn't read {LOCK_FILE}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| anyhow!("{LOCK_FILE} isn't valid: {e}"))
}

fn read_config(dir: &Path) -> Result<ConfigFile> {
    match std::fs::read_to_string(dir.join(CONFIG_FILE)) {
        Ok(text) => {
            serde_json::from_str(&text).map_err(|e| anyhow!("{CONFIG_FILE} isn't valid: {e}"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ConfigFile::default()),
        Err(e) => Err(anyhow!("Couldn't read {CONFIG_FILE}: {e}")),
    }
}

fn pack_name(dir: &Path, config: &ConfigFile) -> String {
    let name = config.name.trim();
    if !name.is_empty() {
        return name.to_string();
    }
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Modpack".into())
}

fn type_of(kind: &str) -> Option<ProjectType> {
    match kind.to_ascii_uppercase().as_str() {
        "MOD" | "MODS" => Some(ProjectType::Mod),
        "RESOURCE_PACK" | "RESOURCE_PACKS" => Some(ProjectType::Resourcepack),
        "SHADER" | "SHADER_PACKS" => Some(ProjectType::Shader),
        "DATA_PACK" | "DATA_PACKS" => Some(ProjectType::Datapack),
        _ => None,
    }
}

fn platform(key: &str) -> Option<ProviderId> {
    match key.to_ascii_lowercase().as_str() {
        "modrinth" => Some(ProviderId::Modrinth),
        "curseforge" => Some(ProviderId::Curseforge),
        _ => None,
    }
}

fn names_of(project: &Project) -> Vec<String> {
    project
        .slug
        .values()
        .chain(project.name.values())
        .chain(project.id.values())
        .chain(project.aliases.iter().flatten())
        .map(|s| s.to_lowercase())
        .collect()
}

fn config_for<'a>(config: &'a ConfigFile, project: &Project) -> Option<&'a ProjectConfig> {
    let names = names_of(project);
    config
        .projects
        .iter()
        .find(|(key, _)| names.contains(&key.to_lowercase()))
        .map(|(_, c)| c)
}

fn display_name(project: &Project, preferred: ProviderId) -> String {
    project
        .name
        .get(preferred.as_str())
        .or_else(|| project.name.values().next())
        .or_else(|| project.slug.values().next())
        .cloned()
        .unwrap_or_else(|| "Unnamed project".into())
}

fn side_of(raw: Option<&str>) -> &'static str {
    match raw.map(str::to_ascii_lowercase).as_deref() {
        Some("client") => "client",
        Some("server") => "server",
        Some("both") => "both",
        _ => "auto",
    }
}

fn item_from(
    project: &Project,
    config: Option<&ProjectConfig>,
    project_type: ProjectType,
    target: Option<&str>,
) -> Option<ContentItem> {
    let mut sources = BTreeMap::new();
    for file in &project.files {
        let Some(provider) = platform(&file.platform) else {
            continue;
        };
        let project_id = Some(file.parent_id.clone())
            .filter(|id| !id.is_empty())
            .or_else(|| project.id.get(provider.as_str()).cloned());
        sources.insert(
            provider,
            SourceFile {
                id: project_id,
                slug: project
                    .slug
                    .get(provider.as_str())
                    .cloned()
                    .unwrap_or_default(),
                version_id: file.id.clone(),
                ..Default::default()
            },
        );
    }
    for (key, id) in &project.id {
        let Some(provider) = platform(key) else {
            continue;
        };
        sources.entry(provider).or_insert_with(|| SourceFile {
            id: Some(id.clone()),
            slug: project.slug.get(key).cloned().unwrap_or_default(),
            ..Default::default()
        });
    }
    let wants_curseforge = target.is_some_and(|t| t.eq_ignore_ascii_case("curseforge"));
    let preferred = match (
        sources.contains_key(&ProviderId::Modrinth),
        sources.contains_key(&ProviderId::Curseforge),
    ) {
        (true, true) if wants_curseforge => ProviderId::Curseforge,
        (true, _) => ProviderId::Modrinth,
        (false, true) => ProviderId::Curseforge,
        (false, false) => {
            let file = project
                .files
                .iter()
                .find(|f| f.url.as_deref().is_some_and(|u| !u.is_empty()))?;
            let hashes = file.hashes.clone().unwrap_or_default();
            sources.insert(
                ProviderId::Url,
                SourceFile {
                    url: file.url.clone(),
                    slug: project.slug.values().next().cloned().unwrap_or_default(),
                    filename: content::plain_filename(&file.file_name),
                    sha1: hashes.get("sha1").cloned(),
                    sha512: hashes.get("sha512").cloned(),
                    ..Default::default()
                },
            );
            ProviderId::Url
        }
    };
    let strategy = config
        .and_then(|c| c.update_strategy.clone())
        .or_else(|| project.update_strategy.clone());
    let redistributable = config
        .and_then(|c| c.redistributable)
        .or(project.redistributable)
        .unwrap_or(true);
    let side = side_of(config.and_then(|c| c.side.as_deref()));
    let (mut client_side, mut server_side) = ("required".to_string(), "required".to_string());
    content::apply_side(side, &mut client_side, &mut server_side);
    Some(ContentItem {
        name: display_name(project, preferred),
        project_type,
        file: String::new(),
        side: side.into(),
        explicit: true,
        disabled: false,
        pinned: strategy.is_some_and(|s| s.eq_ignore_ascii_case("none")),
        preferred,
        sources,
        dependents: Vec::new(),
        optional_dependents: Vec::new(),
        no_bundle: !redistributable,
        client_side,
        server_side,
    })
}

fn rules(patterns: &[String]) -> Vec<Rule> {
    patterns
        .iter()
        .filter_map(|raw| {
            let raw = raw.trim().replace('\\', "/");
            let (keep, pattern) = match raw.strip_prefix('!') {
                Some(rest) => (false, rest.to_string()),
                None => (true, raw),
            };
            let pattern = pattern.trim_start_matches("./").trim_end_matches('/');
            if pattern.is_empty() {
                return None;
            }
            let glob = globset::GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .or_else(|_| Glob::new(&globset::escape(pattern)))
                .ok()?;
            Some(Rule {
                glob: glob.compile_matcher(),
                keep,
            })
        })
        .collect()
}

fn selected(rules: &[Rule], rel: &str) -> bool {
    let mut paths: Vec<&str> = vec![rel];
    let mut rest = rel;
    while let Some((parent, _)) = rest.rsplit_once('/') {
        paths.push(parent);
        rest = parent;
    }
    let mut keep = false;
    for rule in rules {
        if paths.iter().any(|p| rule.glob.is_match(p)) {
            keep = rule.keep;
        }
    }
    keep
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            if rel == ".git" || rel == MANUAL_DIR || rel == instance::OVERRIDES_DIR {
                continue;
            }
            walk(root, &path, out);
        } else if path.is_file() {
            out.push((path, rel));
        }
    }
}

fn override_files(src: &Path, config: &ConfigFile) -> Vec<(PathBuf, String, &'static str)> {
    let common = rules(&config.overrides);
    let server = rules(&config.server_overrides);
    let client = rules(&config.client_overrides);
    let mut found: Vec<(PathBuf, String, &'static str)> = Vec::new();
    if !common.is_empty() || !server.is_empty() || !client.is_empty() {
        let mut files = Vec::new();
        walk(src, src, &mut files);
        for (path, rel) in files {
            if rel == CONFIG_FILE || rel == LOCK_FILE {
                continue;
            }
            let side = match (selected(&client, &rel), selected(&server, &rel)) {
                (true, false) => "client",
                (false, true) => "server",
                (true, true) => "both",
                (false, false) if selected(&common, &rel) => "both",
                _ => continue,
            };
            found.push((path, rel, side));
        }
    }
    for (folder, side) in MANUAL_OVERRIDES {
        let base = src.join(MANUAL_DIR).join(folder);
        let mut files = Vec::new();
        walk(&base, &base, &mut files);
        for (path, rel) in files {
            found.push((path, rel, side));
        }
    }
    found
}

fn folder_for(
    config: &ConfigFile,
    project_type: ProjectType,
    datapacks: &Option<String>,
) -> String {
    let (key, default) = match project_type {
        ProjectType::Mod => ("mods", "mods"),
        ProjectType::Resourcepack => ("resource_packs", "resourcepacks"),
        ProjectType::Shader => ("shader_packs", "shaderpacks"),
        ProjectType::Datapack => {
            return datapacks
                .clone()
                .unwrap_or_else(|| ptype::DEFAULT_DATAPACKS.to_string())
        }
    };
    config
        .paths
        .get(key)
        .and_then(|p| crate::pathsafe::safe_zip_rel(p))
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| default.to_string())
}

pub fn info(dir: &Path) -> Result<ConvertInfo> {
    let lock = read_lock(dir)?;
    let config = read_config(dir)?;
    let content = lock
        .projects
        .iter()
        .filter(|p| type_of(&p.kind).is_some())
        .count();
    Ok(ConvertInfo {
        format: "Pakku".into(),
        name: pack_name(dir, &config),
        content,
        files: override_files(dir, &config).len(),
        git: crate::git::is_repo(dir),
    })
}

pub fn convert(src: &Path, dest: &Path) -> Result<(Manifest, Converted)> {
    let in_place = crate::util::same_dir(src, dest);
    let moving = in_place && crate::git::is_repo(src);
    let lock = read_lock(src)?;
    let config = read_config(src)?;
    let minecraft = lock
        .mc_versions
        .iter()
        .find(|v| !v.trim().is_empty())
        .cloned()
        .ok_or_else(|| anyhow!("{LOCK_FILE} has no Minecraft version."))?;
    let loaders: BTreeMap<String, String> = lock
        .loaders
        .iter()
        .map(|(k, v)| (k.to_lowercase(), v.clone()))
        .collect();
    let (loader, loader_version) = crate::packwiz::loader_of(&loaders);

    let mut datapacks = config
        .paths
        .get("data_packs")
        .and_then(|p| ptype::datapack_folder(p));
    let has_datapacks = lock
        .projects
        .iter()
        .any(|p| type_of(&p.kind) == Some(ProjectType::Datapack));
    if datapacks.is_none() && has_datapacks {
        datapacks = Some(ptype::DEFAULT_DATAPACKS.to_string());
    }

    std::fs::create_dir_all(dest)?;
    let mut report = Converted {
        from: "Pakku".into(),
        kept_source: in_place && !moving,
        ..Default::default()
    };
    let mut taken: HashSet<(ProjectType, String)> = HashSet::new();
    let mut excluded: Vec<String> = Vec::new();
    let mut fetched: Vec<PathBuf> = Vec::new();

    for project in &lock.projects {
        let project_config = config_for(&config, project);
        let kind = project_config
            .and_then(|c| c.kind.as_deref())
            .unwrap_or(&project.kind);
        let label = project
            .name
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| "Unnamed project".into());
        let Some(project_type) = type_of(kind) else {
            let reason = if kind.to_ascii_uppercase().starts_with("WORLD") {
                "worlds aren't supported".to_string()
            } else {
                format!("“{kind}” isn't a supported type")
            };
            report.skipped.push(format!("{label}: {reason}"));
            continue;
        };
        let Some(mut item) = item_from(
            project,
            project_config,
            project_type,
            lock.target.as_deref(),
        ) else {
            report.skipped.push(format!(
                "{label}: no Modrinth, CurseForge, or download link"
            ));
            continue;
        };
        let stem = content::file_stem(&item);
        let mut candidate = stem.clone();
        let mut n = 2;
        while !taken.insert((project_type, candidate.clone())) {
            candidate = format!("{stem}-{n}");
            n += 1;
        }
        item.file = candidate;
        content::add_item(dest, &item)?;
        report.content += 1;
        let exported = project_config
            .and_then(|c| c.export)
            .or(project.export)
            .unwrap_or(true);
        if !exported {
            packignore::add_pattern(
                dest,
                IgnoreKind::Export,
                &format!("/{}", content::item_rel_path(&item)),
            )?;
            excluded.push(item.name.clone());
        }
        let folder = folder_for(&config, project_type, &datapacks);
        for file in &project.files {
            let name = content::plain_filename(&file.file_name);
            if !name.is_empty() {
                fetched.push(src.join(&folder).join(name));
            }
        }
    }
    report.excluded = excluded;

    let overrides = instance::overrides_dir(dest);
    let mut sides: BTreeMap<String, String> = BTreeMap::new();
    let mut emptied: Vec<PathBuf> = Vec::new();
    for (path, rel, side) in override_files(src, &config) {
        let Some(target) = crate::pathsafe::safe_zip_rel(&rel) else {
            report.skipped.push(format!("{rel}: outside the pack"));
            continue;
        };
        let target_rel = target.to_string_lossy().replace('\\', "/");
        place(&path, &overrides.join(&target), moving)?;
        if moving {
            if let Some(parent) = path.parent() {
                emptied.push(parent.to_path_buf());
            }
        }
        if side != "both" {
            sides.insert(target_rel, side.to_string());
        }
        report.files += 1;
    }

    let mf = Manifest {
        name: pack_name(src, &config),
        version: Some(config.version.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "1.0.0".into()),
        minecraft,
        loader,
        loader_version,
        channel: manifest::Channel::Release,
        datapacks,
        override_sides: (!sides.is_empty()).then_some(sides),
        packweave_md: true,
        format_version: 0,
    };
    manifest::write(dest, &mf)?;
    if !in_place {
        copy_docs(src, dest, &mut report.skipped)?;
    }
    if moving {
        for file in [src.join(CONFIG_FILE), src.join(LOCK_FILE)]
            .into_iter()
            .chain(fetched)
        {
            if !file.is_file() {
                continue;
            }
            if let Err(e) = crate::util::delete_file(&file) {
                let name = file.strip_prefix(src).unwrap_or(&file).display();
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
