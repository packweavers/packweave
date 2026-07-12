use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::content::{self, ContentItem};
use crate::curseforge::{self, CurseForge};
use crate::instance::{self, InstanceMod};
use crate::launchers;
use crate::lockfile::{self, target_dir, LockedMod, Lockfile, SourceFile};
use crate::manifest::{self, Manifest};
use crate::modrinth::Modrinth;
use crate::packlocal;
use crate::providers::ProviderId;
use crate::ptype::ProjectType;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub bound: bool,
    pub instance_dir: Option<String>,
    pub mods: Vec<ModChange>,
    pub files: Vec<FileChange>,
    pub env: Option<EnvChange>,
    pub in_sync: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvChange {
    pub pack_minecraft: String,
    pub pack_loader: String,
    pub pack_loader_version: Option<String>,
    pub instance_minecraft: Option<String>,
    pub instance_loader: Option<String>,
    pub instance_loader_version: Option<String>,
    pub writable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub kind: String,
    pub path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOp {
    pub target: String,
    pub kind: String,
    pub direction: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub provider: Option<ProviderId>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub instance_version_id: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub rel_path: Option<String>,
    #[serde(default)]
    pub project_type: ProjectType,
}

pub async fn status(
    mr: &Modrinth,
    cf: &CurseForge,
    pack_dir: &Path,
) -> Result<SyncReport> {
    let local = packlocal::read(pack_dir);
    let instance_dir = local.instance_dir.clone();
    let instance = match &instance_dir {
        Some(d) => PathBuf::from(d),
        None => {
            return Ok(SyncReport {
                bound: false,
                instance_dir: None,
                mods: vec![],
                files: vec![],
                env: None,
                in_sync: true,
            })
        }
    };

    let matcher = instance::ignore_matcher(pack_dir);
    let user_matcher = instance::user_ignore_matcher(pack_dir);
    let (pack_mods, disabled_mods): (Vec<LockedMod>, Vec<LockedMod>) =
        lockfile::read(pack_dir)
            .map(|l| l.mods.into_iter().partition(|m| !m.disabled))
            .unwrap_or_default();
    let pack_by_project: HashMap<&str, &LockedMod> = pack_mods
        .iter()
        .map(|m| (m.project_id.as_str(), m))
        .collect();
    let pack_by_sha1: HashMap<&str, &LockedMod> = pack_mods
        .iter()
        .filter_map(|m| m.sha1().filter(|s| !s.is_empty()).map(|s| (s, m)))
        .collect();
    let disabled_by_project: HashMap<&str, &LockedMod> = disabled_mods
        .iter()
        .map(|m| (m.project_id.as_str(), m))
        .collect();
    let disabled_by_sha1: HashMap<&str, &LockedMod> = disabled_mods
        .iter()
        .filter_map(|m| m.sha1().filter(|s| !s.is_empty()).map(|s| (s, m)))
        .collect();

    let inst_mods = instance::scan_content(&instance);
    let hashes: Vec<String> =
        inst_mods.iter().map(|m| m.sha1.clone()).collect();
    let found = mr.versions_from_hashes(&hashes).await?;

    let cf_query: Vec<u32> = inst_mods
        .iter()
        .filter(|im| {
            !pack_by_sha1.contains_key(im.sha1.as_str())
                && !found.contains_key(&im.sha1)
        })
        .map(|im| im.murmur2)
        .collect();
    let cf_found = cf.fingerprints(&cf_query).await.unwrap_or_default();

    let override_content =
        instance::scan_content(&instance::overrides_dir(pack_dir));
    let override_by_path: HashMap<String, String> = override_content
        .iter()
        .map(|m| (m.rel_path.clone(), m.sha1.clone()))
        .collect();

    struct InstHit<'a> {
        im: &'a InstanceMod,
        project_id: String,
        version_id: String,
        version_number: String,
        name: String,
        provider: ProviderId,
    }

    let mut matched_pids: HashSet<String> = HashSet::new();
    let mut hits: Vec<InstHit> = Vec::new();
    let mut mods: Vec<ModChange> = Vec::new();

    for im in &inst_mods {
        if let Some(pm) = pack_by_sha1.get(im.sha1.as_str()) {
            matched_pids.insert(pm.project_id.clone());
            continue;
        }
        if let Some(dm) = disabled_by_sha1.get(im.sha1.as_str()) {
            mods.push(disabled_change(dm, im, None, None));
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
        match override_by_path.get(&im.rel_path) {
            Some(ov_hash) => {
                if ov_hash != &im.sha1 {
                    mods.push(ModChange {
                        kind: "local_changed".into(),
                        name: im.filename.clone(),
                        project_id: None,
                        slug: None,
                        provider: None,
                        project_type: type_from_rel(&im.rel_path),
                        instance_version_id: None,
                        instance_version: None,
                        pack_version: None,
                        filename: Some(im.filename.clone()),
                        rel_path: Some(im.rel_path.clone()),
                        dependency: false,
                    });
                }
            }
            None => {
                if instance::is_ignored(&user_matcher, &im.rel_path, false) {
                    continue;
                }
                mods.push(ModChange {
                    kind: "unknown".into(),
                    name: im.filename.clone(),
                    project_id: None,
                    slug: None,
                    provider: None,
                    project_type: type_from_rel(&im.rel_path),
                    instance_version_id: None,
                    instance_version: None,
                    pack_version: None,
                    filename: Some(im.filename.clone()),
                    rel_path: Some(im.rel_path.clone()),
                    dependency: false,
                });
            }
        }
    }

    for hit in &hits {
        match pack_by_project.get(hit.project_id.as_str()) {
            Some(pm) => {
                matched_pids.insert(hit.project_id.clone());
                if pm.version_id() != hit.version_id {
                    mods.push(ModChange {
                        kind: "version_diff".into(),
                        name: pm.name.clone(),
                        project_id: Some(hit.project_id.clone()),
                        slug: Some(pm.slug.clone()),
                        provider: Some(pm.preferred),
                        project_type: pm.project_type,
                        instance_version_id: Some(hit.version_id.clone()),
                        instance_version: Some(hit.version_number.clone()),
                        pack_version: Some(pm.version_number().to_string()),
                        filename: Some(hit.im.filename.clone()),
                        rel_path: Some(hit.im.rel_path.clone()),
                        dependency: pm.dependency_type
                            == lockfile::DepType::Dependency,
                    });
                }
            }
            None => {
                if let Some(dm) =
                    disabled_by_project.get(hit.project_id.as_str())
                {
                    mods.push(disabled_change(
                        dm,
                        hit.im,
                        Some(hit.version_id.clone()),
                        Some(hit.version_number.clone()),
                    ));
                } else {
                    mods.push(ModChange {
                        kind: "instance_only".into(),
                        name: hit.name.clone(),
                        project_id: Some(hit.project_id.clone()),
                        slug: None,
                        provider: Some(hit.provider),
                        project_type: type_from_rel(&hit.im.rel_path),
                        instance_version_id: Some(hit.version_id.clone()),
                        instance_version: Some(hit.version_number.clone()),
                        pack_version: None,
                        filename: Some(hit.im.filename.clone()),
                        rel_path: Some(hit.im.rel_path.clone()),
                        dependency: false,
                    });
                }
            }
        }
    }

    for pm in &pack_mods {
        if !matched_pids.contains(&pm.project_id) {
            mods.push(ModChange {
                kind: "pack_only".into(),
                name: pm.name.clone(),
                project_id: Some(pm.project_id.clone()),
                slug: Some(pm.slug.clone()),
                provider: Some(pm.preferred),
                project_type: pm.project_type,
                instance_version_id: None,
                instance_version: None,
                pack_version: Some(pm.version_number().to_string()),
                filename: Some(pm.filename().to_string()),
                rel_path: None,
                dependency: pm.dependency_type == lockfile::DepType::Dependency,
            });
        }
    }

    let inst_paths: HashSet<&str> =
        inst_mods.iter().map(|m| m.rel_path.as_str()).collect();
    for ov in &override_content {
        if !inst_paths.contains(ov.rel_path.as_str()) {
            mods.push(ModChange {
                kind: "local_only".into(),
                name: ov.filename.clone(),
                project_id: None,
                slug: None,
                provider: None,
                project_type: type_from_rel(&ov.rel_path),
                instance_version_id: None,
                instance_version: None,
                pack_version: None,
                filename: Some(ov.filename.clone()),
                rel_path: Some(ov.rel_path.clone()),
                dependency: false,
            });
        }
    }

    let unnamed_mr: Vec<String> = mods
        .iter()
        .filter(|m| {
            m.kind == "instance_only"
                && m.provider == Some(ProviderId::Modrinth)
        })
        .filter_map(|m| m.project_id.clone())
        .collect();
    if !unnamed_mr.is_empty() {
        let projects = mr.projects_bulk(&unnamed_mr).await?;
        let titles: HashMap<String, (String, String)> = projects
            .into_iter()
            .map(|p| (p.id, (p.title, p.slug)))
            .collect();
        for m in mods.iter_mut() {
            if let Some(pid) = &m.project_id {
                if let Some((title, slug)) = titles.get(pid) {
                    m.name = title.clone();
                    m.slug = Some(slug.clone());
                }
            }
        }
    }

    let unnamed_cf: Vec<String> = mods
        .iter()
        .filter(|m| {
            m.kind == "instance_only"
                && m.provider == Some(ProviderId::Curseforge)
        })
        .filter_map(|m| m.project_id.clone())
        .collect();
    if !unnamed_cf.is_empty() {
        let named = cf.mods_named(&unnamed_cf).await;
        for m in mods.iter_mut() {
            if m.kind == "instance_only"
                && m.provider == Some(ProviderId::Curseforge)
            {
                if let Some(pid) = &m.project_id {
                    if let Some((name, slug)) = named.get(pid) {
                        if !name.is_empty() {
                            m.name = name.clone();
                        }
                        if !slug.is_empty() {
                            m.slug = Some(slug.clone());
                        }
                    }
                }
            }
        }
    }

    let attrs = instance::GitAttributes::load(pack_dir);
    let inst_files = instance::scan_tracked(&instance, &matcher, &attrs);
    let pack_files = instance::scan_overrides(pack_dir, &matcher, &attrs);
    let pack_map: HashMap<&str, &str> = pack_files
        .iter()
        .map(|f| (f.path.as_str(), f.sha1.as_str()))
        .collect();
    let inst_set: HashSet<&str> =
        inst_files.iter().map(|f| f.path.as_str()).collect();

    let mut files: Vec<FileChange> = Vec::new();
    for f in &inst_files {
        match pack_map.get(f.path.as_str()) {
            Some(pack_sha) => {
                if *pack_sha != f.sha1 {
                    files.push(FileChange {
                        kind: "changed".into(),
                        path: f.path.clone(),
                    });
                }
            }
            None => files.push(FileChange {
                kind: "new".into(),
                path: f.path.clone(),
            }),
        }
    }
    for f in &pack_files {
        if !inst_set.contains(f.path.as_str()) {
            files.push(FileChange {
                kind: "removed".into(),
                path: f.path.clone(),
            });
        }
    }

    mods.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.cmp(&b.name)));
    files.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.path.cmp(&b.path)));

    let env = manifest::read(pack_dir)
        .ok()
        .and_then(|m| env_change(&m, &instance));

    let in_sync = mods.is_empty() && files.is_empty() && env.is_none();
    Ok(SyncReport {
        bound: true,
        instance_dir,
        mods,
        files,
        env,
        in_sync,
    })
}

fn env_change(manifest: &Manifest, instance: &Path) -> Option<EnvChange> {
    let (inst_mc, inst_loader, inst_lver) = launchers::read_env(instance);
    if inst_mc.is_none() && inst_loader.is_none() {
        return None;
    }
    let loader = inst_loader.clone().unwrap_or_else(|| "vanilla".into());
    let mc_diff = inst_mc.as_deref().unwrap_or_default() != manifest.minecraft;
    let loader_diff = loader != manifest.loader;
    let lver_diff = manifest
        .loader_version
        .as_ref()
        .is_some_and(|pv| Some(pv) != inst_lver.as_ref());
    if !mc_diff && !loader_diff && !lver_diff {
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
    })
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
                    if let (Some(pid), Some(vid)) =
                        (&op.project_id, &op.instance_version_id)
                    {
                        let provider =
                            op.provider.unwrap_or(ProviderId::Modrinth);
                        let mut sources = BTreeMap::new();
                        sources.insert(
                            provider,
                            SourceFile {
                                id: Some(pid.clone()),
                                pin: Some(vid.clone()),
                                slug: op.slug.clone().unwrap_or_default(),
                                ..Default::default()
                            },
                        );
                        content::add_item(
                            pack_dir,
                            &ContentItem {
                                name: op
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| pid.clone()),
                                project_type: op.project_type,
                                side: "auto".into(),
                                explicit: true,
                                disabled: false,
                                preferred: provider,
                                sources,
                                dependents: Vec::new(),
                                client_side: "required".into(),
                                server_side: "required".into(),
                            },
                        )?;
                    }
                }
                "version_diff" => {
                    if let (Some(pid), Some(vid)) =
                        (&op.project_id, &op.instance_version_id)
                    {
                        content::set_pin(pack_dir, pid, Some(vid.clone()))?;
                    }
                }
                "pack_only" => {
                    if let Some(pid) = &op.project_id {
                        content::remove_item(pack_dir, pid)?;
                    }
                }
                "unknown" | "local_changed" => {
                    if let Some(rel) = &op.rel_path {
                        instance::copy_into_overrides(
                            pack_dir, &instance, rel,
                        )?;
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
            ("mod", "ignore") => {
                if let Some(rel) = op.rel_path.clone().or_else(|| {
                    op.filename.as_ref().map(|f| format!("mods/{f}"))
                }) {
                    instance::add_ignore(pack_dir, &rel)?;
                }
            }
            ("env", "pull") => {
                let (inst_mc, inst_loader, inst_lver) =
                    launchers::read_env(&instance);
                let mut manifest = manifest::read(pack_dir)?;
                if let Some(mc) = inst_mc {
                    manifest.minecraft = mc;
                }
                manifest.loader =
                    inst_loader.unwrap_or_else(|| "vanilla".into());
                manifest.loader_version = inst_lver;
                manifest::write(pack_dir, &manifest)?;
            }
            ("file", "pull") => {
                if let Some(path) = &op.path {
                    match op.kind.as_str() {
                        "new" | "changed" => instance::copy_into_overrides(
                            pack_dir, &instance, path,
                        )?,
                        "removed" => {
                            instance::remove_from_overrides(pack_dir, path)?
                        }
                        _ => {}
                    }
                }
            }
            ("file", "ignore") => {
                if let Some(path) = &op.path {
                    instance::add_ignore(pack_dir, path)?;
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

    for op in ops {
        match (op.target.as_str(), op.direction.as_str()) {
            ("mod", "push") => match op.kind.as_str() {
                "pack_only" | "version_diff" => {
                    if let Some(pid) = &op.project_id {
                        if let Some(lm) =
                            lock.mods.iter().find(|m| &m.project_id == pid)
                        {
                            push_mod(mr, &instance, lm).await?;
                            if op.kind == "version_diff" {
                                let new_rel = format!(
                                    "{}/{}",
                                    target_dir(lm.project_type),
                                    lm.filename()
                                );
                                if let Some(rel) = &op.rel_path {
                                    if rel != &new_rel {
                                        instance::remove_from_instance(
                                            &instance, rel,
                                        )?;
                                    }
                                }
                            }
                        }
                    }
                }
                "instance_only" | "disabled" => {
                    if let Some(rel) = &op.rel_path {
                        instance::remove_from_instance(&instance, rel)?;
                    } else if let Some(fname) = &op.filename {
                        instance::remove_from_instance(
                            &instance,
                            &format!("mods/{}", fname),
                        )?;
                    }
                }
                "local_changed" | "local_only" => {
                    if let Some(rel) = &op.rel_path {
                        instance::copy_into_instance(pack_dir, &instance, rel)?;
                    }
                }
                "unknown" => {
                    if let Some(rel) = &op.rel_path {
                        instance::remove_from_instance(&instance, rel)?;
                    }
                }
                _ => {}
            },
            ("file", "push") => {
                if let Some(path) = &op.path {
                    match op.kind.as_str() {
                        "changed" | "removed" => instance::copy_into_instance(
                            pack_dir, &instance, path,
                        )?,
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

async fn push_mod(
    mr: &Modrinth,
    instance: &Path,
    lm: &LockedMod,
) -> Result<()> {
    if lm.download_url().is_empty() {
        return Ok(());
    }
    let bytes = mr.download(lm.download_url()).await?;
    let dir = instance.join(target_dir(lm.project_type));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(lm.filename()), &bytes)?;
    Ok(())
}

fn disabled_change(
    dm: &LockedMod,
    im: &InstanceMod,
    version_id: Option<String>,
    version: Option<String>,
) -> ModChange {
    ModChange {
        kind: "disabled".into(),
        name: dm.name.clone(),
        project_id: Some(dm.project_id.clone()),
        slug: Some(dm.slug.clone()),
        provider: Some(dm.preferred),
        project_type: dm.project_type,
        instance_version_id: version_id,
        instance_version: version,
        pack_version: None,
        filename: Some(im.filename.clone()),
        rel_path: Some(im.rel_path.clone()),
        dependency: false,
    }
}

fn type_from_rel(rel: &str) -> ProjectType {
    ProjectType::from_instance_path(rel)
}

pub fn auto_push_file(
    pack_dir: &Path,
    rel: &str,
    original: Option<&str>,
) -> Result<bool> {
    let local = packlocal::read(pack_dir);
    let instance = match local.instance_dir.as_ref().map(PathBuf::from) {
        Some(d) => d,
        None => return Ok(false),
    };
    let ov_rel = match rel.strip_prefix("overrides/") {
        Some(r) if !r.is_empty() => r,
        _ => return Ok(false),
    };
    let conflicting = match std::fs::read(instance.join(ov_rel)) {
        Ok(bytes) => match original {
            Some(orig) => {
                instance::normalize_eol(&bytes)
                    != instance::normalize_eol(orig.as_bytes())
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
        _ => {
            return Ok(
                "(binary or unreadable file, no text diff available)".into()
            )
        }
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
