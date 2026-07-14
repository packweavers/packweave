use anyhow::{anyhow, Result};
use futures::stream::StreamExt;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::content::ContentItem;
use crate::curseforge::CurseForge;
use crate::lockfile::SourceFile;
use crate::manifest::Manifest;
use crate::modrinth::Modrinth;
use crate::providers::{self, DepKind, ProviderId, Resolved};
use crate::ptype::ProjectType;

pub const ISSUE_MISSING_VERSION: &str = "missing_version";
pub const ISSUE_MISSING_DEPENDENCY: &str = "missing_dependency";
pub const ISSUE_DISABLED_DEPENDENCY: &str = "disabled_dependency";
pub const ISSUE_PROVIDER: &str = "provider";

const FETCH_CONCURRENCY: usize = 8;

pub struct ResolveOutput {
    pub locked: Vec<ContentItem>,
    pub issues: Vec<ResolveIssue>,
    pub incompatible: Vec<Incompat>,
    pub optional: Vec<OptionalDep>,
    pub conflicts: Vec<VersionConflict>,
    pub names: HashMap<String, ProjectMeta>,
}

pub struct VersionConflict {
    pub project_id: String,
    pub provider: ProviderId,
    pub picks: Vec<ConflictPick>,
}

pub struct ConflictPick {
    pub version: String,
    pub requested_by: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactReport {
    pub changes: Vec<ImpactChange>,
    pub problems: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactChange {
    pub kind: String,
    pub name: String,
    pub project_type: ProjectType,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub dependency: bool,
}

pub fn diff_locked(
    current: &[ContentItem],
    next: &[ContentItem],
) -> Vec<ImpactChange> {
    let cur: HashMap<String, &ContentItem> =
        current.iter().map(|i| (i.project_id(), i)).collect();
    let nxt: HashMap<String, &ContentItem> =
        next.iter().map(|i| (i.project_id(), i)).collect();
    let ver = |i: &ContentItem| {
        i.active()
            .map(|s| s.version_number.clone())
            .filter(|s| !s.is_empty())
    };
    let mut out: Vec<ImpactChange> = Vec::new();
    for (id, ni) in &nxt {
        match cur.get(id) {
            Some(ci) => {
                let (cv, nv) = (ver(ci), ver(ni));
                if cv != nv {
                    out.push(ImpactChange {
                        kind: if ni.explicit {
                            "update".into()
                        } else {
                            "dep_changed".into()
                        },
                        name: ni.name.clone(),
                        project_type: ni.project_type,
                        from_version: cv,
                        to_version: nv,
                        dependency: !ni.explicit,
                    });
                }
            }
            None => out.push(ImpactChange {
                kind: "added".into(),
                name: ni.name.clone(),
                project_type: ni.project_type,
                from_version: None,
                to_version: ver(ni),
                dependency: !ni.explicit,
            }),
        }
    }
    for (id, ci) in &cur {
        if !nxt.contains_key(id) {
            out.push(ImpactChange {
                kind: "removed".into(),
                name: ci.name.clone(),
                project_type: ci.project_type,
                from_version: ver(ci),
                to_version: None,
                dependency: !ci.explicit,
            });
        }
    }
    out.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out
}

pub fn impact_problems(out: &ResolveOutput) -> Vec<String> {
    let name = |id: &str| {
        out.names
            .get(id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let present: HashSet<String> =
        out.locked.iter().map(|i| i.project_id()).collect();
    let mut v = Vec::new();
    for c in &out.conflicts {
        let versions: Vec<String> =
            c.picks.iter().map(|p| p.version.clone()).collect();
        v.push(format!(
            "{} needs conflicting versions: {}",
            name(&c.project_id),
            versions.join(" vs ")
        ));
    }
    for iss in &out.issues {
        if iss.kind == ISSUE_MISSING_DEPENDENCY {
            v.push(format!("Missing dependency: {}", name(&iss.project_id)));
        }
    }
    for inc in &out.incompatible {
        if present.contains(&inc.project_id)
            && present.contains(&inc.with_project_id)
        {
            v.push(format!(
                "{} is incompatible with {}",
                name(&inc.project_id),
                name(&inc.with_project_id)
            ));
        }
    }
    v
}

#[derive(Clone)]
pub struct ProjectMeta {
    pub name: String,
    pub slug: String,
}

pub struct ResolveIssue {
    pub kind: String,
    pub project_id: String,
    pub provider: ProviderId,
    pub detail: String,
    pub required_by: Vec<String>,
}

pub struct Incompat {
    pub project_id: String,
    pub provider: ProviderId,
    pub with_project_id: String,
}

pub struct OptionalDep {
    pub project_id: String,
    pub provider: ProviderId,
    pub required_by: String,
}

struct Target {
    provider: ProviderId,
    id: Option<String>,
    url: Option<String>,
    path: Option<String>,
    pin: Option<String>,
    side: String,
    explicit: bool,
    required_by: Option<String>,
    project_type: ProjectType,
    name: String,
}

impl Target {
    fn key(&self) -> String {
        self.id
            .clone()
            .or_else(|| self.url.clone())
            .or_else(|| self.path.clone())
            .filter(|x| !x.is_empty())
            .unwrap_or_else(|| self.name.clone())
    }
}

fn src_ident(s: &SourceFile) -> Option<String> {
    s.id.clone()
        .or_else(|| s.url.clone())
        .or_else(|| s.path.clone())
        .filter(|x| !x.is_empty())
}

pub async fn resolve(
    mr: &Modrinth,
    cf: &CurseForge,
    manifest: &Manifest,
    authored: &[ContentItem],
    excluded: &HashSet<String>,
) -> Result<ResolveOutput> {
    let mc = &manifest.minecraft;
    let loader = &manifest.loader;
    let channel = manifest.channel.rank();

    let mut chosen: HashMap<String, (Target, Resolved)> = HashMap::new();
    let mut leaves: Vec<ContentItem> = Vec::new();
    let mut explicit_set: HashSet<String> = HashSet::new();
    let mut dependents: HashMap<String, HashSet<String>> = HashMap::new();
    let mut issues: Vec<ResolveIssue> = Vec::new();
    let mut incompatible: Vec<Incompat> = Vec::new();
    let mut optional: Vec<OptionalDep> = Vec::new();
    let mut disabled_edges: Vec<(String, String, ProviderId)> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut alt_map: HashMap<String, Vec<(ProviderId, SourceFile)>> =
        HashMap::new();
    let mut pin_requests: HashMap<String, BTreeMap<String, BTreeSet<String>>> =
        HashMap::new();

    let mut frontier: Vec<Target> = Vec::new();
    for item in authored {
        let pref = item.preferred;
        let psrc = item.sources.get(&pref).cloned().unwrap_or_default();
        let key = src_ident(&psrc).unwrap_or_else(|| item.name.clone());
        explicit_set.insert(key.clone());
        let alts: Vec<(ProviderId, SourceFile)> = item
            .sources
            .iter()
            .filter(|(p, _)| **p != pref)
            .map(|(p, s)| (*p, s.clone()))
            .collect();
        if !alts.is_empty() {
            alt_map.insert(key.clone(), alts);
        }
        frontier.push(Target {
            provider: pref,
            id: psrc.id.clone(),
            url: psrc.url.clone(),
            path: psrc.path.clone(),
            pin: psrc.pin.clone(),
            side: item.side.clone(),
            explicit: true,
            required_by: None,
            project_type: item.project_type,
            name: item.name.clone(),
        });
    }

    while !frontier.is_empty() {
        let mut layer: Vec<Target> = Vec::new();
        for t in frontier.drain(..) {
            let k = t.key();
            if let Some(parent) = &t.required_by {
                dependents
                    .entry(k.clone())
                    .or_default()
                    .insert(parent.clone());
            }
            if let Some(pin) = t.pin.clone().filter(|p| !p.is_empty()) {
                let by = t.required_by.clone().unwrap_or_else(|| "pack".into());
                pin_requests
                    .entry(k.clone())
                    .or_default()
                    .entry(pin)
                    .or_default()
                    .insert(by);
            }
            if visited.contains(&k) {
                continue;
            }
            visited.insert(k);
            layer.push(t);
        }
        if layer.is_empty() {
            break;
        }

        let mut api_targets: Vec<Target> = Vec::new();
        for t in layer {
            match t.provider {
                ProviderId::Url => leaves.push(leaf_from_url(&t)),
                ProviderId::Local => {
                    if let Some(item) = leaf_from_local(&t) {
                        leaves.push(item);
                    } else {
                        issues.push(ResolveIssue {
                            kind: ISSUE_PROVIDER.into(),
                            project_id: t.key(),
                            provider: t.provider,
                            detail: format!(
                                "Local file missing for {}",
                                t.name
                            ),
                            required_by: t
                                .required_by
                                .iter()
                                .cloned()
                                .collect(),
                        });
                    }
                }
                _ => api_targets.push(t),
            }
        }

        let fetches = api_targets.into_iter().map(|t| async move {
            let lf = if t.project_type == ProjectType::Mod {
                loader.as_str()
            } else {
                ""
            };
            let res = match providers::by_id(t.provider, mr, cf) {
                Some(p) => {
                    p.resolve(
                        t.id.as_deref().unwrap_or(""),
                        t.pin.as_deref(),
                        mc,
                        lf,
                        t.project_type,
                        channel,
                    )
                    .await
                }
                None => Err(anyhow!(
                    "Provider '{}' is not supported yet ({}).",
                    t.provider,
                    t.name
                )),
            };
            (t, res)
        });
        let results: Vec<(Target, Result<Resolved>)> =
            futures::stream::iter(fetches)
                .buffer_unordered(FETCH_CONCURRENCY)
                .collect()
                .await;

        let mut next: Vec<Target> = Vec::new();
        for (target, res) in results {
            let key = target.key();
            match res {
                Ok(r) => {
                    for dep in &r.deps {
                        match dep.kind {
                            DepKind::Required => {
                                if excluded.contains(&dep.id) {
                                    disabled_edges.push((
                                        key.clone(),
                                        dep.id.clone(),
                                        dep.provider,
                                    ));
                                } else {
                                    next.push(dep_target(
                                        dep.provider,
                                        dep.id.clone(),
                                        dep.pin.clone(),
                                        key.clone(),
                                    ));
                                }
                            }
                            DepKind::Optional => optional.push(OptionalDep {
                                project_id: dep.id.clone(),
                                provider: dep.provider,
                                required_by: key.clone(),
                            }),
                            DepKind::Incompatible => {
                                incompatible.push(Incompat {
                                    project_id: key.clone(),
                                    provider: target.provider,
                                    with_project_id: dep.id.clone(),
                                })
                            }
                        }
                    }
                    chosen.insert(key, (target, r));
                }
                Err(e) => {
                    let mut required_by: Vec<String> = dependents
                        .get(&key)
                        .map(|s| s.iter().cloned().collect())
                        .unwrap_or_default();
                    required_by.sort();
                    issues.push(ResolveIssue {
                        kind: if target.explicit {
                            ISSUE_MISSING_VERSION.into()
                        } else {
                            ISSUE_MISSING_DEPENDENCY.into()
                        },
                        project_id: key,
                        provider: target.provider,
                        detail: format!("{e}"),
                        required_by,
                    });
                    if target.explicit {
                        leaves.push(unresolved_leaf(&target));
                    }
                }
            }
        }

        frontier = next;
    }

    let mut by_disabled: HashMap<(String, ProviderId), BTreeSet<String>> =
        HashMap::new();
    for (parent, dep_id, prov) in disabled_edges {
        if chosen.contains_key(&dep_id) {
            continue;
        }
        by_disabled
            .entry((dep_id, prov))
            .or_default()
            .insert(parent);
    }
    for ((dep_id, prov), parents) in by_disabled {
        issues.push(ResolveIssue {
            kind: ISSUE_DISABLED_DEPENDENCY.into(),
            project_id: dep_id,
            provider: prov,
            detail: String::new(),
            required_by: parents.into_iter().collect(),
        });
    }

    let mut conflicts: Vec<VersionConflict> = Vec::new();
    for (key, pins) in &pin_requests {
        if pins.len() < 2 {
            continue;
        }
        let (provider, ptype, win_pin, win_num) = match chosen.get(key) {
            Some((t, r)) => (
                t.provider,
                t.project_type,
                t.pin.clone(),
                r.version_number.clone(),
            ),
            None => {
                (ProviderId::Modrinth, ProjectType::Mod, None, String::new())
            }
        };
        let lf = if ptype == ProjectType::Mod {
            loader.as_str()
        } else {
            ""
        };
        let mut picks = Vec::new();
        for (pin, by) in pins {
            let version = if win_pin.as_deref() == Some(pin.as_str())
                && !win_num.is_empty()
            {
                win_num.clone()
            } else if let Some(p) = providers::by_id(provider, mr, cf) {
                p.resolve(key, Some(pin), mc, lf, ptype, channel)
                    .await
                    .map(|r| r.version_number)
                    .ok()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| pin.clone())
            } else {
                pin.clone()
            };
            picks.push(ConflictPick {
                version,
                requested_by: by.iter().cloned().collect(),
            });
        }
        picks.sort_by(|a, b| a.version.cmp(&b.version));
        conflicts.push(VersionConflict {
            project_id: key.clone(),
            provider,
            picks,
        });
    }
    conflicts.sort_by(|a, b| a.project_id.cmp(&b.project_id));

    let mut meta: HashMap<String, ProjectMeta> = HashMap::new();
    for (key, (_t, r)) in &chosen {
        meta.insert(
            key.clone(),
            ProjectMeta {
                name: r.name.clone(),
                slug: r.slug.clone(),
            },
        );
    }

    let mut need: HashMap<ProviderId, HashSet<String>> = HashMap::new();
    for o in &optional {
        if !meta.contains_key(&o.project_id) {
            need.entry(o.provider)
                .or_default()
                .insert(o.project_id.clone());
        }
    }
    for i in &incompatible {
        if !meta.contains_key(&i.with_project_id) {
            need.entry(i.provider)
                .or_default()
                .insert(i.with_project_id.clone());
        }
    }
    for iss in &issues {
        if !meta.contains_key(&iss.project_id) {
            need.entry(iss.provider)
                .or_default()
                .insert(iss.project_id.clone());
        }
    }
    for c in &conflicts {
        if !meta.contains_key(&c.project_id) {
            need.entry(c.provider)
                .or_default()
                .insert(c.project_id.clone());
        }
    }
    for (prov, ids) in need {
        if let Some(p) = providers::by_id(prov, mr, cf) {
            let ids: Vec<String> = ids.into_iter().collect();
            for (id, (name, slug)) in p.names(&ids).await {
                meta.entry(id).or_insert(ProjectMeta { name, slug });
            }
        }
    }

    let mut locked: Vec<ContentItem> = leaves;
    for (key, (target, r)) in &chosen {
        let (mut client_side, mut server_side) = (
            r.client_side.clone().unwrap_or_else(|| "required".into()),
            r.server_side.clone().unwrap_or_else(|| "required".into()),
        );
        match target.side.as_str() {
            "client" => server_side = "unsupported".into(),
            "server" => client_side = "unsupported".into(),
            "both" => {
                if client_side == "unsupported" {
                    client_side = "optional".into();
                }
                if server_side == "unsupported" {
                    server_side = "optional".into();
                }
            }
            _ => {}
        }
        let mut deps: Vec<String> = dependents
            .get(key)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        deps.sort();

        let mut sources = BTreeMap::new();
        sources.insert(target.provider, source_file(target, r));
        locked.push(ContentItem {
            name: if !r.name.is_empty() {
                r.name.clone()
            } else {
                target.name.clone()
            },
            project_type: target.project_type,
            side: target.side.clone(),
            explicit: explicit_set.contains(key),
            disabled: false,
            preferred: target.provider,
            sources,
            dependents: deps,
            client_side,
            server_side,
        });
    }

    for item in locked.iter_mut() {
        if !item.explicit {
            continue;
        }
        let key = item
            .sources
            .get(&item.preferred)
            .and_then(src_ident)
            .unwrap_or_else(|| item.name.clone());
        let alts = match alt_map.get(&key) {
            Some(a) => a.clone(),
            None => continue,
        };
        let lf = if item.project_type == ProjectType::Mod {
            loader.as_str()
        } else {
            ""
        };
        for (provider, src) in alts {
            if item.sources.contains_key(&provider) {
                continue;
            }
            let resolved = match providers::by_id(provider, mr, cf) {
                Some(p) => p
                    .resolve(
                        src.id.as_deref().unwrap_or(""),
                        src.pin.as_deref(),
                        mc,
                        lf,
                        item.project_type,
                        channel,
                    )
                    .await
                    .ok(),
                None => None,
            };
            let entry = match resolved {
                Some(r) => SourceFile {
                    id: src.id.clone(),
                    url: src.url.clone(),
                    path: src.path.clone(),
                    pin: src.pin.clone(),
                    slug: r.slug,
                    version_id: r.version_id,
                    version_number: r.version_number,
                    filename: r.filename,
                    download_url: r.download_url,
                    sha1: r.sha1,
                    sha512: r.sha512,
                    file_size: r.file_size,
                },
                None => SourceFile {
                    id: src.id.clone(),
                    url: src.url.clone(),
                    path: src.path.clone(),
                    pin: src.pin.clone(),
                    ..Default::default()
                },
            };
            item.sources.insert(provider, entry);
        }
    }

    locked.sort_by(|a, b| {
        b.explicit
            .cmp(&a.explicit)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(ResolveOutput {
        locked,
        issues,
        incompatible,
        optional,
        conflicts,
        names: meta,
    })
}

fn dep_target(
    provider: ProviderId,
    id: String,
    pin: Option<String>,
    parent: String,
) -> Target {
    Target {
        provider,
        id: Some(id),
        url: None,
        path: None,
        pin,
        side: "auto".into(),
        explicit: false,
        required_by: Some(parent),
        project_type: ProjectType::Mod,
        name: String::new(),
    }
}

fn source_file(t: &Target, r: &Resolved) -> SourceFile {
    SourceFile {
        id: t.id.clone(),
        url: t.url.clone(),
        path: t.path.clone(),
        pin: if t.explicit { t.pin.clone() } else { None },
        slug: r.slug.clone(),
        version_id: r.version_id.clone(),
        version_number: r.version_number.clone(),
        filename: r.filename.clone(),
        download_url: r.download_url.clone(),
        sha1: r.sha1.clone(),
        sha512: r.sha512.clone(),
        file_size: r.file_size,
    }
}

fn sides_for(side: &str) -> (String, String) {
    match side {
        "client" => ("required".into(), "unsupported".into()),
        "server" => ("unsupported".into(), "required".into()),
        _ => ("required".into(), "required".into()),
    }
}

fn base_item(
    t: &Target,
    sources: BTreeMap<ProviderId, SourceFile>,
) -> ContentItem {
    let (client_side, server_side) = sides_for(&t.side);
    ContentItem {
        name: t.name.clone(),
        project_type: t.project_type,
        side: t.side.clone(),
        explicit: t.explicit,
        disabled: false,
        preferred: t.provider,
        sources,
        dependents: t.required_by.iter().cloned().collect(),
        client_side,
        server_side,
    }
}

fn leaf_from_url(t: &Target) -> ContentItem {
    let url = t.url.clone().unwrap_or_default();
    let filename = t
        .url
        .as_deref()
        .and_then(|u| u.rsplit('/').next())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}.jar", t.name));
    let mut sources = BTreeMap::new();
    sources.insert(
        ProviderId::Url,
        SourceFile {
            url: Some(url.clone()),
            filename,
            download_url: url,
            ..Default::default()
        },
    );
    base_item(t, sources)
}

fn leaf_from_local(t: &Target) -> Option<ContentItem> {
    let path = t.path.clone()?;
    let filename = path.rsplit('/').next().unwrap_or(&path).to_string();
    let mut sources = BTreeMap::new();
    sources.insert(
        ProviderId::Local,
        SourceFile {
            path: Some(path),
            filename,
            ..Default::default()
        },
    );
    Some(base_item(t, sources))
}

fn unresolved_leaf(t: &Target) -> ContentItem {
    let mut sources = BTreeMap::new();
    sources.insert(
        t.provider,
        SourceFile {
            id: t.id.clone(),
            url: t.url.clone(),
            path: t.path.clone(),
            pin: t.pin.clone(),
            ..Default::default()
        },
    );
    base_item(t, sources)
}
