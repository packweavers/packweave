use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::content::{self, ContentItem};
use crate::curseforge::{self, CurseForge};
use crate::instance::{self, InstanceMod, SyncIgnore};
use crate::launchers;
use crate::lockfile::{self, LockedMod, Lockfile, SourceFile};
use crate::manifest::{self, Manifest};
use crate::modrinth::Modrinth;
use crate::packlocal;
use crate::providers::ProviderId;
use crate::ptype::{Folders, ProjectType};
use crate::syncbase::{self, Baseline};
use crate::syncignore::{self, Ignored};
use ts_rs::TS;

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "types.ts", rename = "SyncSide")]
pub enum Side {
    Push,
    Pull,
    Both,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "types.ts", rename = "ChangeWord")]
pub enum Change {
    Added,
    Changed,
    Removed,
}

#[derive(Default, Clone)]
struct Fingerprint {
    key: String,
    pack: Option<String>,
    inst: Option<String>,
    pack_file: Option<PathBuf>,
    inst_file: Option<PathBuf>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct SyncReport {
    pub bound: bool,
    pub instance_dir: Option<String>,
    pub mods: Vec<ModChange>,
    pub files: Vec<FileChange>,
    pub env: Option<EnvChange>,
    pub ignored: Vec<Ignored>,
    pub in_sync: bool,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct EnvChange {
    pub pack_minecraft: String,
    pub pack_loader: String,
    pub pack_loader_version: Option<String>,
    pub instance_minecraft: Option<String>,
    pub instance_loader: Option<String>,
    pub instance_loader_version: Option<String>,
    pub writable: bool,
    pub side: Side,
    pub key: String,
    pub ignored: bool,
    #[serde(skip)]
    fp: Fingerprint,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ModChange {
    pub kind: String,
    pub name: String,
    pub project_id: Option<String>,
    pub slug: Option<String>,
    pub provider: Option<ProviderId>,
    pub project_type: ProjectType,
    pub instance_version_id: Option<String>,
    pub instance_version: Option<String>,
    pub pack_version: Option<String>,
    pub filename: Option<String>,
    pub rel_path: Option<String>,
    pub dependency: bool,
    pub side: Side,
    pub change: Change,
    pub modified: Option<i64>,
    pub key: String,
    pub ignored: bool,
    #[serde(skip)]
    fp: Fingerprint,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct FileChange {
    pub kind: String,
    pub path: String,
    pub side: Side,
    pub change: Change,
    pub modified: Option<i64>,
    pub key: String,
    pub ignored: bool,
    #[serde(skip)]
    fp: Fingerprint,
}

#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct SyncOp {
    #[ts(type = "'mod' | 'file' | 'env'")]
    pub target: String,
    pub kind: String,
    #[ts(type = "'push' | 'pull'")]
    pub direction: String,
    #[serde(default)]
    #[ts(optional)]
    pub path: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub project_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub slug: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub provider: Option<ProviderId>,
    #[serde(default)]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub instance_version_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub filename: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub rel_path: Option<String>,
    #[serde(default)]
    #[ts(as = "Option<ProjectType>", optional)]
    pub project_type: ProjectType,
}

struct InstHit<'a> {
    im: &'a InstanceMod,
    project_id: String,
    version_id: String,
    version_number: String,
    name: String,
    provider: ProviderId,
}

struct PackIndex<'a> {
    by_project: HashMap<&'a str, &'a LockedMod>,
    by_sha1: HashMap<&'a str, &'a LockedMod>,
    disabled_by_project: HashMap<&'a str, &'a LockedMod>,
    disabled_by_sha1: HashMap<&'a str, &'a LockedMod>,
}

impl<'a> PackIndex<'a> {
    fn build(enabled: &'a [LockedMod], disabled: &'a [LockedMod]) -> Self {
        let by_hash = |mods: &'a [LockedMod]| {
            mods.iter()
                .filter_map(|m| m.sha1().filter(|s| !s.is_empty()).map(|s| (s, m)))
                .collect()
        };
        let by_id = |mods: &'a [LockedMod]| {
            let mut map: HashMap<&'a str, &'a LockedMod> =
                mods.iter().map(|m| (m.project_id.as_str(), m)).collect();
            for m in mods {
                for id in m.sources.values().filter_map(|s| s.id.as_deref()) {
                    if !id.is_empty() {
                        map.entry(id).or_insert(m);
                    }
                }
            }
            map
        };
        PackIndex {
            by_project: by_id(enabled),
            by_sha1: by_hash(enabled),
            disabled_by_project: by_id(disabled),
            disabled_by_sha1: by_hash(disabled),
        }
    }
}

struct Scan<'a> {
    pack_dir: &'a Path,
    instance: &'a Path,
    mods: Vec<ModChange>,
    files: Vec<FileChange>,
    agreed: Vec<(String, String)>,
}

impl Scan<'_> {
    fn pack_mod_file(&self, m: &LockedMod) -> PathBuf {
        self.pack_dir.join(content::locked_rel_path(m))
    }

    fn instance_file(&self, rel: &str) -> PathBuf {
        self.instance.join(rel)
    }

    fn override_file(&self, rel: &str) -> PathBuf {
        instance::overrides_dir(self.pack_dir).join(rel)
    }

    fn agree(&mut self, key: String, value: String) {
        self.agreed.push((key, value));
    }
}

fn mod_key(project_id: &str) -> String {
    format!("mod:{project_id}")
}

fn legacy_keys<'a>(mods: impl Iterator<Item = &'a LockedMod>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for m in mods {
        for id in m.sources.values().filter_map(|s| s.id.as_deref()) {
            if !id.is_empty() {
                out.push((mod_key(id), mod_key(&m.key)));
            }
        }
    }
    out
}

fn jar_key(rel: &str) -> String {
    format!("jar:{rel}")
}

fn file_key(path: &str) -> String {
    format!("file:{path}")
}

fn blank_mod(kind: &str, name: String, project_type: ProjectType) -> ModChange {
    ModChange {
        kind: kind.into(),
        name,
        project_id: None,
        slug: None,
        provider: None,
        project_type,
        instance_version_id: None,
        instance_version: None,
        pack_version: None,
        filename: None,
        rel_path: None,
        dependency: false,
        side: Side::Push,
        change: Change::Changed,
        modified: None,
        key: String::new(),
        ignored: false,
        fp: Fingerprint::default(),
    }
}

fn is_dependency(m: &LockedMod) -> bool {
    m.dependency_type == lockfile::DepType::Dependency
}

pub async fn status(mr: &Modrinth, cf: &CurseForge, pack_dir: &Path) -> Result<SyncReport> {
    let local = packlocal::read(pack_dir);
    let Some(instance_dir) = local.instance_dir.clone() else {
        return Ok(SyncReport {
            bound: false,
            instance_dir: None,
            mods: vec![],
            files: vec![],
            env: None,
            ignored: vec![],
            in_sync: true,
        });
    };
    let instance = PathBuf::from(&instance_dir);
    if !instance.is_dir() {
        return Err(anyhow!(
            "MISSING_INSTANCE:The instance folder is missing: {instance_dir}"
        ));
    }
    let ignore = SyncIgnore::load(pack_dir);
    let (enabled, disabled): (Vec<LockedMod>, Vec<LockedMod>) = lockfile::read(pack_dir)
        .map(|l| l.mods.into_iter().partition(|m| !m.disabled))
        .unwrap_or_default();
    let legacy = legacy_keys(enabled.iter().chain(&disabled));
    syncignore::rename(&instance, &legacy)?;
    let ignored = syncignore::load(&instance);
    let index = PackIndex::build(&enabled, &disabled);
    let folders = Folders::load(pack_dir);
    let not_ignored = |m: &InstanceMod| !ignore.ignores_user(&m.rel_path, false);
    let inst_jars: Vec<InstanceMod> = instance::scan_content(&instance, &folders)
        .into_iter()
        .filter(not_ignored)
        .collect();
    let override_jars: Vec<InstanceMod> =
        instance::scan_content(&instance::overrides_dir(pack_dir), &folders)
            .into_iter()
            .filter(not_ignored)
            .collect();

    let mut scan = Scan {
        pack_dir,
        instance: &instance,
        mods: Vec::new(),
        files: Vec::new(),
        agreed: Vec::new(),
    };
    let (hits, mut matched) =
        identify_jars(mr, cf, &index, &inst_jars, &override_jars, &mut scan).await?;
    compare_hits(&hits, &index, &mut matched, &mut scan);
    pack_only_changes(&enabled, &matched, &mut scan);
    override_only_changes(&override_jars, &inst_jars, &mut scan);
    name_instance_only(mr, cf, &mut scan.mods).await?;
    diff_files(&ignore, &mut scan);
    let env = manifest::read(pack_dir)
        .ok()
        .and_then(|m| env_change(&m, &instance, &mut scan));

    let Scan {
        mut mods,
        mut files,
        agreed,
        ..
    } = scan;
    let mut env = env;
    let mut base = syncbase::load(pack_dir, &instance);
    for (old, new) in &legacy {
        if let Some(value) = base.remove(old) {
            base.entry(new.clone()).or_insert(value);
        }
    }
    for m in mods.iter_mut() {
        m.side = classify(&m.fp, &base);
        m.change = change_for(&m.kind, m.side);
        m.modified = modified(&m.fp, m.side);
        m.key = m.fp.key.clone();
        m.ignored = syncignore::covered(&ignored, &m.key);
    }
    for f in files.iter_mut() {
        f.side = classify(&f.fp, &base);
        f.change = change_for(&f.kind, f.side);
        f.modified = modified(&f.fp, f.side);
        f.key = f.fp.key.clone();
        f.ignored = syncignore::covered(&ignored, &f.key);
    }
    if let Some(e) = env.as_mut() {
        e.side = classify(&e.fp, &base);
        e.key = e.fp.key.clone();
        e.ignored = syncignore::covered(&ignored, &e.key);
    }
    let observed = mods
        .iter()
        .map(|m| &m.fp)
        .chain(files.iter().map(|f| &f.fp))
        .chain(env.iter().map(|e| &e.fp));
    syncbase::save(pack_dir, &instance, &learn(&base, observed, &agreed))?;

    mods.sort_by_key(|m| m.name.to_lowercase());
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let in_sync = mods.iter().all(|m| m.ignored)
        && files.iter().all(|f| f.ignored)
        && env.as_ref().is_none_or(|e| e.ignored);
    Ok(SyncReport {
        bound: true,
        instance_dir: Some(instance_dir),
        mods,
        files,
        env,
        ignored,
        in_sync,
    })
}

fn classify(fp: &Fingerprint, base: &Baseline) -> Side {
    if let Some(agreed) = base.get(&fp.key) {
        let agreed = Some(agreed);
        if fp.pack.as_ref() == agreed {
            return Side::Pull;
        }
        if fp.inst.as_ref() == agreed {
            return Side::Push;
        }
        return Side::Both;
    }
    match (&fp.pack, &fp.inst) {
        (Some(_), None) => Side::Push,
        (None, Some(_)) => Side::Pull,
        _ => {
            let pack = fp.pack_file.as_deref().and_then(mtime);
            let inst = fp.inst_file.as_deref().and_then(mtime);
            match (pack, inst) {
                (Some(p), Some(i)) if i > p => Side::Pull,
                _ => Side::Push,
            }
        }
    }
}

fn change_for(kind: &str, side: Side) -> Change {
    let instance_has_it = matches!(kind, "instance_only" | "disabled" | "unknown" | "new");
    let pack_has_it = matches!(kind, "pack_only" | "local_only" | "removed");
    match side {
        Side::Pull if instance_has_it => Change::Added,
        Side::Pull if pack_has_it => Change::Removed,
        Side::Push if pack_has_it => Change::Added,
        Side::Push if instance_has_it => Change::Removed,
        _ => Change::Changed,
    }
}

fn modified(fp: &Fingerprint, side: Side) -> Option<i64> {
    let pack = fp.pack_file.as_deref().and_then(mtime);
    let inst = fp.inst_file.as_deref().and_then(mtime);
    match side {
        Side::Push => pack,
        Side::Pull => inst,
        Side::Both => pack.max(inst),
    }
}

fn mtime(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

fn learn<'a>(
    base: &Baseline,
    observed: impl Iterator<Item = &'a Fingerprint>,
    agreed: &[(String, String)],
) -> Baseline {
    let mut next = Baseline::new();
    for fp in observed {
        if let Some(value) = base.get(&fp.key) {
            next.insert(fp.key.clone(), value.clone());
        }
    }
    for (key, value) in agreed {
        next.insert(key.clone(), value.clone());
    }
    next
}

async fn identify_jars<'a>(
    mr: &Modrinth,
    cf: &CurseForge,
    index: &PackIndex<'_>,
    inst_jars: &'a [InstanceMod],
    override_jars: &[InstanceMod],
    scan: &mut Scan<'_>,
) -> Result<(Vec<InstHit<'a>>, HashSet<String>)> {
    let override_by_path: HashMap<&str, &str> = override_jars
        .iter()
        .map(|m| (m.rel_path.as_str(), m.sha1.as_str()))
        .collect();
    let unknown: Vec<&InstanceMod> = inst_jars
        .iter()
        .filter(|im| {
            !index.by_sha1.contains_key(im.sha1.as_str())
                && !index.disabled_by_sha1.contains_key(im.sha1.as_str())
                && !override_by_path.contains_key(im.rel_path.as_str())
        })
        .collect();
    let hashes: Vec<String> = unknown.iter().map(|m| m.sha1.clone()).collect();
    let found = mr.versions_by_hashes(&hashes).await?;
    let cf_query: Vec<u32> = unknown
        .iter()
        .filter(|im| !found.contains_key(&im.sha1))
        .map(|im| im.murmur2)
        .collect();
    let cf_found = cf.fingerprints(&cf_query).await.unwrap_or_default();

    let mut matched: HashSet<String> = HashSet::new();
    let mut hits: Vec<InstHit> = Vec::new();
    for im in inst_jars {
        if let Some(pm) = index.by_sha1.get(im.sha1.as_str()) {
            matched.insert(pm.project_id.clone());
            scan.agree(mod_key(&pm.key), pm.version_id().to_string());
            continue;
        }
        if let Some(dm) = index.disabled_by_sha1.get(im.sha1.as_str()) {
            let change = disabled_change(dm, im, None, None, scan);
            scan.mods.push(change);
            continue;
        }
        if let Some(ov_sha) = override_by_path.get(im.rel_path.as_str()) {
            if *ov_sha == im.sha1 {
                scan.agree(jar_key(&im.rel_path), im.sha1.clone());
            } else {
                let mut change = loose_jar_change("local_changed", im);
                change.fp = Fingerprint {
                    key: jar_key(&im.rel_path),
                    pack: Some(ov_sha.to_string()),
                    inst: Some(im.sha1.clone()),
                    pack_file: Some(scan.override_file(&im.rel_path)),
                    inst_file: Some(scan.instance_file(&im.rel_path)),
                };
                scan.mods.push(change);
            }
            continue;
        }
        if let Some(ver) = found.get(&im.sha1) {
            hits.push(InstHit {
                im,
                project_id: ver.project_id.clone(),
                version_id: ver.id.clone(),
                version_number: ver.version_number.clone(),
                name: ver.name.clone(),
                provider: ProviderId::Modrinth,
            });
            continue;
        }
        if let Some((mod_id, file)) = cf_found.get(&(im.murmur2 as u64)) {
            hits.push(InstHit {
                im,
                project_id: mod_id.to_string(),
                version_id: file.id.to_string(),
                version_number: curseforge::file_version(file),
                name: im.filename.clone(),
                provider: ProviderId::Curseforge,
            });
            continue;
        }
        let mut change = loose_jar_change("unknown", im);
        change.fp = Fingerprint {
            key: jar_key(&im.rel_path),
            pack: None,
            inst: Some(im.sha1.clone()),
            pack_file: None,
            inst_file: Some(scan.instance_file(&im.rel_path)),
        };
        scan.mods.push(change);
    }
    Ok((hits, matched))
}

fn loose_jar_change(kind: &str, im: &InstanceMod) -> ModChange {
    let mut change = blank_mod(kind, im.filename.clone(), im.project_type);
    change.filename = Some(im.filename.clone());
    change.rel_path = Some(im.rel_path.clone());
    change
}

fn compare_hits(
    hits: &[InstHit<'_>],
    index: &PackIndex<'_>,
    matched: &mut HashSet<String>,
    scan: &mut Scan<'_>,
) {
    for hit in hits {
        if let Some(pm) = index.by_project.get(hit.project_id.as_str()) {
            matched.insert(pm.project_id.clone());
            if pm.version_id() == hit.version_id {
                scan.agree(mod_key(&pm.key), hit.version_id.clone());
                continue;
            }
            let mut change = blank_mod("version_diff", pm.name.clone(), pm.project_type);
            change.project_id = Some(pm.project_id.clone());
            change.slug = Some(pm.slug.clone());
            change.provider = Some(hit.provider);
            change.instance_version_id = Some(hit.version_id.clone());
            change.instance_version = Some(hit.version_number.clone());
            change.pack_version = Some(pm.version_number().to_string());
            change.filename = Some(hit.im.filename.clone());
            change.rel_path = Some(hit.im.rel_path.clone());
            change.dependency = is_dependency(pm);
            change.fp = Fingerprint {
                key: mod_key(&pm.key),
                pack: Some(pm.version_id().to_string()),
                inst: Some(hit.version_id.clone()),
                pack_file: Some(scan.pack_mod_file(pm)),
                inst_file: Some(scan.instance_file(&hit.im.rel_path)),
            };
            scan.mods.push(change);
            continue;
        }
        if let Some(dm) = index.disabled_by_project.get(hit.project_id.as_str()) {
            let change = disabled_change(
                dm,
                hit.im,
                Some(hit.version_id.clone()),
                Some(hit.version_number.clone()),
                scan,
            );
            scan.mods.push(change);
            continue;
        }
        let mut change = blank_mod("instance_only", hit.name.clone(), hit.im.project_type);
        change.project_id = Some(hit.project_id.clone());
        change.provider = Some(hit.provider);
        change.instance_version_id = Some(hit.version_id.clone());
        change.instance_version = Some(hit.version_number.clone());
        change.filename = Some(hit.im.filename.clone());
        change.rel_path = Some(hit.im.rel_path.clone());
        change.fp = Fingerprint {
            key: mod_key(&hit.project_id),
            pack: None,
            inst: Some(hit.version_id.clone()),
            pack_file: None,
            inst_file: Some(scan.instance_file(&hit.im.rel_path)),
        };
        scan.mods.push(change);
    }
}

fn disabled_change(
    dm: &LockedMod,
    im: &InstanceMod,
    version_id: Option<String>,
    version: Option<String>,
    scan: &Scan<'_>,
) -> ModChange {
    let mut change = blank_mod("disabled", dm.name.clone(), dm.project_type);
    change.project_id = Some(dm.project_id.clone());
    change.slug = Some(dm.slug.clone());
    change.provider = Some(dm.preferred);
    change.instance_version = version;
    change.fp = Fingerprint {
        key: mod_key(&dm.key),
        pack: None,
        inst: Some(
            version_id
                .clone()
                .unwrap_or_else(|| dm.version_id().to_string()),
        ),
        pack_file: Some(scan.pack_mod_file(dm)),
        inst_file: Some(scan.instance_file(&im.rel_path)),
    };
    change.instance_version_id = version_id;
    change.filename = Some(im.filename.clone());
    change.rel_path = Some(im.rel_path.clone());
    change
}

fn pack_only_changes(enabled: &[LockedMod], matched: &HashSet<String>, scan: &mut Scan<'_>) {
    for pm in enabled
        .iter()
        .filter(|pm| !matched.contains(&pm.project_id))
    {
        let mut change = blank_mod("pack_only", pm.name.clone(), pm.project_type);
        change.project_id = Some(pm.project_id.clone());
        change.slug = Some(pm.slug.clone());
        change.provider = Some(pm.preferred);
        change.pack_version = Some(pm.version_number().to_string());
        change.filename = Some(pm.filename().to_string());
        change.dependency = is_dependency(pm);
        change.fp = Fingerprint {
            key: mod_key(&pm.key),
            pack: Some(pm.version_id().to_string()),
            inst: None,
            pack_file: Some(scan.pack_mod_file(pm)),
            inst_file: None,
        };
        scan.mods.push(change);
    }
}

fn override_only_changes(
    override_jars: &[InstanceMod],
    inst_jars: &[InstanceMod],
    scan: &mut Scan<'_>,
) {
    let inst_paths: HashSet<&str> = inst_jars.iter().map(|m| m.rel_path.as_str()).collect();
    for ov in override_jars
        .iter()
        .filter(|ov| !inst_paths.contains(ov.rel_path.as_str()))
    {
        let mut change = loose_jar_change("local_only", ov);
        change.fp = Fingerprint {
            key: jar_key(&ov.rel_path),
            pack: Some(ov.sha1.clone()),
            inst: None,
            pack_file: Some(scan.override_file(&ov.rel_path)),
            inst_file: None,
        };
        scan.mods.push(change);
    }
}

async fn name_instance_only(mr: &Modrinth, cf: &CurseForge, mods: &mut [ModChange]) -> Result<()> {
    fn ids_for(mods: &[ModChange], provider: ProviderId) -> Vec<String> {
        mods.iter()
            .filter(|m| m.kind == "instance_only" && m.provider == Some(provider))
            .filter_map(|m| m.project_id.clone())
            .collect()
    }

    let unnamed_mr = ids_for(mods, ProviderId::Modrinth);
    if !unnamed_mr.is_empty() {
        let titles: HashMap<String, (String, String)> = mr
            .projects_bulk(&unnamed_mr)
            .await?
            .into_iter()
            .map(|p| (p.id, (p.title, p.slug)))
            .collect();
        for m in mods.iter_mut() {
            if let Some((title, slug)) = m.project_id.as_ref().and_then(|pid| titles.get(pid)) {
                m.name = title.clone();
                m.slug = Some(slug.clone());
            }
        }
    }

    let unnamed_cf = ids_for(mods, ProviderId::Curseforge);
    if !unnamed_cf.is_empty() {
        let named = cf.mods_named(&unnamed_cf).await;
        for m in mods.iter_mut() {
            if m.kind != "instance_only" || m.provider != Some(ProviderId::Curseforge) {
                continue;
            }
            if let Some((name, slug)) = m.project_id.as_ref().and_then(|pid| named.get(pid)) {
                if !name.is_empty() {
                    m.name = name.clone();
                }
                if !slug.is_empty() {
                    m.slug = Some(slug.clone());
                }
            }
        }
    }
    Ok(())
}

fn diff_files(ignore: &SyncIgnore, scan: &mut Scan<'_>) {
    let attrs = instance::GitAttributes::load(scan.pack_dir);
    let inst_files = instance::scan_tracked(scan.instance, ignore, &attrs);
    let pack_files = instance::scan_overrides(scan.pack_dir, ignore, &attrs);
    let pack_map: HashMap<&str, &str> = pack_files
        .iter()
        .map(|f| (f.path.as_str(), f.sha1.as_str()))
        .collect();
    let inst_set: HashSet<&str> = inst_files.iter().map(|f| f.path.as_str()).collect();

    for f in &inst_files {
        let pack_sha = pack_map.get(f.path.as_str()).copied();
        if pack_sha == Some(f.sha1.as_str()) {
            scan.agree(file_key(&f.path), f.sha1.clone());
            continue;
        }
        let kind = if pack_sha.is_some() { "changed" } else { "new" };
        let fp = Fingerprint {
            key: file_key(&f.path),
            pack: pack_sha.map(str::to_string),
            inst: Some(f.sha1.clone()),
            pack_file: pack_sha.map(|_| scan.override_file(&f.path)),
            inst_file: Some(scan.instance_file(&f.path)),
        };
        scan.files.push(file_change(kind, &f.path, fp));
    }
    for f in pack_files
        .iter()
        .filter(|f| !inst_set.contains(f.path.as_str()))
    {
        let fp = Fingerprint {
            key: file_key(&f.path),
            pack: Some(f.sha1.clone()),
            inst: None,
            pack_file: Some(scan.override_file(&f.path)),
            inst_file: None,
        };
        scan.files.push(file_change("removed", &f.path, fp));
    }
}

fn file_change(kind: &str, path: &str, fp: Fingerprint) -> FileChange {
    FileChange {
        kind: kind.into(),
        path: path.to_string(),
        side: Side::Push,
        change: Change::Changed,
        modified: None,
        key: String::new(),
        ignored: false,
        fp,
    }
}

fn env_change(manifest: &Manifest, instance: &Path, scan: &mut Scan<'_>) -> Option<EnvChange> {
    let (inst_mc, inst_loader, inst_lver) = launchers::read_env(instance);
    if inst_mc.is_none() && inst_loader.is_none() {
        return None;
    }
    let loader = inst_loader.clone().unwrap_or_else(|| "vanilla".into());
    let inst_value = format!(
        "{}|{}|{}",
        inst_mc.as_deref().unwrap_or_default(),
        loader,
        inst_lver.as_deref().unwrap_or_default()
    );
    let pack_lver = manifest
        .loader_version
        .clone()
        .or_else(|| inst_lver.clone())
        .unwrap_or_default();
    let pack_value = format!("{}|{}|{}", manifest.minecraft, manifest.loader, pack_lver);
    if pack_value == inst_value {
        scan.agree("env".into(), pack_value);
        return None;
    }
    Some(EnvChange {
        pack_minecraft: manifest.minecraft.clone(),
        pack_loader: manifest.loader.clone(),
        pack_loader_version: manifest.loader_version.clone(),
        instance_minecraft: inst_mc,
        instance_loader: inst_loader,
        instance_loader_version: inst_lver,
        writable: launchers::env_writable(instance),
        side: Side::Push,
        key: String::new(),
        ignored: false,
        fp: Fingerprint {
            key: "env".into(),
            pack: Some(pack_value),
            inst: Some(inst_value),
            pack_file: Some(manifest::path_for(scan.pack_dir)),
            inst_file: None,
        },
    })
}

pub fn affected_paths(pack_dir: &Path, ops: &[SyncOp], lock: Option<&Lockfile>) -> Vec<PathBuf> {
    let local = packlocal::read(pack_dir);
    let Some(instance) = local.instance_dir.map(PathBuf::from) else {
        return Vec::new();
    };
    let overrides = instance::overrides_dir(pack_dir);
    let folders = Folders::load(pack_dir);
    let mut out = Vec::new();
    for op in ops {
        let rel = op.rel_path.clone().or_else(|| op.path.clone());
        match (op.target.as_str(), op.direction.as_str()) {
            ("file", "pull") | ("mod", "pull") => {
                if let Some(rel) = rel {
                    out.push(overrides.join(rel));
                }
            }
            ("file", "push") => {
                if let Some(rel) = rel {
                    out.push(instance.join(rel));
                }
            }
            ("mod", "push") => {
                if let Some(rel) = rel {
                    out.push(instance.join(rel));
                }
                let new_jar = lock.and_then(|l| {
                    l.mods
                        .iter()
                        .find(|m| Some(&m.project_id) == op.project_id.as_ref())
                });
                if let Some(m) = new_jar {
                    out.push(
                        instance
                            .join(folders.install_dir(m.project_type))
                            .join(m.filename()),
                    );
                }
            }
            ("env", "push") => {
                for name in [
                    "mmc-pack.json",
                    "../mmc-pack.json",
                    "launcher_profiles.json",
                ] {
                    out.push(instance.join(name));
                }
            }
            _ => {}
        }
    }
    out
}

pub fn apply_pull(pack_dir: &Path, ops: &[SyncOp]) -> Result<()> {
    let local = packlocal::read(pack_dir);
    let instance = local
        .instance_dir
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("No instance linked"))?;

    for op in ops {
        match (op.target.as_str(), op.direction.as_str()) {
            ("mod", "pull") => match op.kind.as_str() {
                "instance_only" => {
                    if let (Some(pid), Some(vid)) = (&op.project_id, &op.instance_version_id) {
                        let provider = op.provider.unwrap_or(ProviderId::Modrinth);
                        let mut sources = BTreeMap::new();
                        sources.insert(
                            provider,
                            SourceFile {
                                id: Some(pid.clone()),
                                version_id: vid.clone(),
                                slug: op.slug.clone().unwrap_or_default(),
                                ..Default::default()
                            },
                        );
                        content::add_item(
                            pack_dir,
                            &ContentItem {
                                name: op.name.clone().unwrap_or_else(|| pid.clone()),
                                project_type: op.project_type,
                                side: "auto".into(),
                                explicit: true,
                                preferred: provider,
                                sources,
                                client_side: "required".into(),
                                server_side: "required".into(),
                                ..Default::default()
                            },
                        )?;
                    }
                }
                "version_diff" => {
                    if let (Some(pid), Some(vid)) = (&op.project_id, &op.instance_version_id) {
                        if let Some(provider) = op.provider {
                            content::set_preferred(pack_dir, pid, provider)?;
                        }
                        content::set_version(pack_dir, pid, vid.clone())?;
                    }
                }
                "pack_only" => {
                    if let Some(pid) = &op.project_id {
                        content::remove_item(pack_dir, pid)?;
                    }
                }
                "unknown" | "local_changed" => {
                    if let Some(rel) = &op.rel_path {
                        instance::copy_into_overrides(pack_dir, &instance, rel)?;
                    }
                }
                "local_only" => {
                    if let Some(rel) = &op.rel_path {
                        instance::remove_from_overrides(pack_dir, rel)?;
                    }
                }
                "disabled" => {
                    if let Some(pid) = &op.project_id {
                        content::set_disabled(pack_dir, pid, false)?;
                    }
                }
                _ => {}
            },
            ("env", "pull") => {
                let (inst_mc, inst_loader, inst_lver) = launchers::read_env(&instance);
                let mut manifest = manifest::read(pack_dir)?;
                if let Some(mc) = inst_mc {
                    manifest.minecraft = mc;
                }
                manifest.loader = inst_loader.unwrap_or_else(|| "vanilla".into());
                manifest.loader_version = inst_lver;
                manifest::write(pack_dir, &manifest)?;
            }
            ("file", "pull") => {
                if let Some(path) = &op.path {
                    match op.kind.as_str() {
                        "new" | "changed" => {
                            instance::copy_into_overrides(pack_dir, &instance, path)?
                        }
                        "removed" => instance::remove_from_overrides(pack_dir, path)?,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

pub async fn apply_push(
    mr: &Modrinth,
    pack_dir: &Path,
    ops: &[SyncOp],
    lock: &Lockfile,
) -> Result<()> {
    let local = packlocal::read(pack_dir);
    let instance = local
        .instance_dir
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("No instance linked"))?;
    let folders = Folders::load(pack_dir);

    for op in ops {
        match (op.target.as_str(), op.direction.as_str()) {
            ("mod", "push") => match op.kind.as_str() {
                "pack_only" | "version_diff" => {
                    let Some(pid) = &op.project_id else {
                        continue;
                    };
                    let Some(lm) = lock.mods.iter().find(|m| &m.project_id == pid) else {
                        continue;
                    };
                    push_mod(mr, &instance, &folders, lm).await?;
                    if op.kind == "version_diff" {
                        let new_rel =
                            format!("{}/{}", folders.install_dir(lm.project_type), lm.filename());
                        if let Some(rel) = &op.rel_path {
                            if rel != &new_rel {
                                instance::remove_from_instance(&instance, rel)?;
                            }
                        }
                    }
                }
                "instance_only" | "disabled" | "unknown" => {
                    if let Some(rel) = &op.rel_path {
                        instance::remove_from_instance(&instance, rel)?;
                    } else if let Some(fname) = &op.filename {
                        instance::remove_from_instance(&instance, &format!("mods/{fname}"))?;
                    }
                }
                "local_changed" | "local_only" => {
                    if let Some(rel) = &op.rel_path {
                        instance::copy_into_instance(pack_dir, &instance, rel)?;
                    }
                }
                _ => {}
            },
            ("file", "push") => {
                if let Some(path) = &op.path {
                    match op.kind.as_str() {
                        "changed" | "removed" => {
                            instance::copy_into_instance(pack_dir, &instance, path)?
                        }
                        "new" => instance::remove_from_instance(&instance, path)?,
                        _ => {}
                    }
                }
            }
            ("env", "push") => {
                launchers::write_env(
                    &instance,
                    &lock.minecraft,
                    &lock.loader,
                    lock.loader_version.as_deref(),
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn push_mod(mr: &Modrinth, instance: &Path, folders: &Folders, lm: &LockedMod) -> Result<()> {
    if lm.download_url().is_empty() {
        return Ok(());
    }
    let bytes = mr.download(lm.download_url()).await?;
    let dir = instance.join(folders.install_dir(lm.project_type));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(lm.filename()), &bytes)?;
    Ok(())
}

pub fn auto_push_file(pack_dir: &Path, rel: &str, original: Option<&str>) -> Result<bool> {
    let local = packlocal::read(pack_dir);
    let instance = match local.instance_dir.as_ref().map(PathBuf::from) {
        Some(d) => d,
        None => return Ok(false),
    };
    let ov_rel = match rel.strip_prefix("overrides/") {
        Some(r) if !r.is_empty() => r,
        _ => return Ok(false),
    };
    if syncignore::covered(&syncignore::load(&instance), &file_key(ov_rel)) {
        return Ok(false);
    }
    let conflicting = match std::fs::read(instance.join(ov_rel)) {
        Ok(bytes) => match original {
            Some(orig) => {
                instance::normalize_eol(&bytes) != instance::normalize_eol(orig.as_bytes())
            }
            None => true,
        },
        Err(_) => false,
    };
    if conflicting {
        return Ok(false);
    }
    instance::copy_into_instance(pack_dir, &instance, ov_rel)?;
    Ok(true)
}

pub fn file_diff(pack_dir: &Path, rel: &str, kind: &str) -> Result<String> {
    let local = packlocal::read(pack_dir);
    let instance = local
        .instance_dir
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("No instance linked"))?;

    let pack_text = read_text(&instance::overrides_dir(pack_dir).join(rel));
    let inst_text = read_text(&instance.join(rel));

    let (old, new) = match kind {
        "new" => (Some(String::new()), inst_text),
        "removed" => (pack_text, Some(String::new())),
        _ => (pack_text, inst_text),
    };
    let (old, new) = match (old, new) {
        (Some(o), Some(n)) => (o, n),
        _ => return Ok("(binary or unreadable file, no text diff available)".into()),
    };

    let diff = TextDiff::from_lines(&old, &new);
    let mut out = String::new();
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            out.push_str("⋯\n");
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => '-',
                    ChangeTag::Insert => '+',
                    ChangeTag::Equal => ' ',
                };
                out.push(sign);
                out.push_str(change.value());
                if !change.value().ends_with('\n') {
                    out.push('\n');
                }
            }
        }
    }
    if out.is_empty() {
        out.push_str("(no textual changes)\n");
    }
    Ok(out)
}

fn read_text(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|s| s.replace("\r\n", "\n").replace('\r', "\n"))
}
