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
use ts_rs::TS;

pub const ISSUE_MISSING_VERSION: &str = "missing_version";
pub const ISSUE_INVALID_VERSION: &str = "invalid_version";
pub const ISSUE_MISSING_DEPENDENCY: &str = "missing_dependency";
pub const ISSUE_DISABLED_DEPENDENCY: &str = "disabled_dependency";
pub const ISSUE_PROVIDER: &str = "provider";
pub const ISSUE_MISSING_FILE: &str = "missing_file";

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
    pub pinned: Option<String>,
    pub current: String,
}

pub struct ConflictPick {
    pub version: String,
    pub requested_by: Vec<String>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ImpactReport {
    pub changes: Vec<ImpactChange>,
    pub problems: Vec<String>,
    pub fixed: Vec<String>,
    pub optional: Vec<Suggested>,
}

#[derive(Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Suggested {
    pub project_id: String,
    pub provider: ProviderId,
    pub name: String,
    pub slug: String,
    pub wanted_by: Vec<String>,
}

pub fn suggestions(out: &ResolveOutput, present: &HashSet<String>) -> Vec<Suggested> {
    let mut wanted: BTreeMap<String, (ProviderId, BTreeSet<String>)> = BTreeMap::new();
    for o in &out.optional {
        if present.contains(&o.project_id) {
            continue;
        }
        wanted
            .entry(o.project_id.clone())
            .or_insert_with(|| (o.provider, BTreeSet::new()))
            .1
            .insert(o.required_by.clone());
    }
    wanted
        .into_iter()
        .map(|(project_id, (provider, by))| {
            let meta = out.names.get(&project_id);
            Suggested {
                name: meta
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| project_id.clone()),
                slug: meta.map(|m| m.slug.clone()).unwrap_or_default(),
                project_id,
                provider,
                wanted_by: by.into_iter().collect(),
            }
        })
        .collect()
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ImpactChange {
    pub kind: String,
    pub name: String,
    pub key: String,
    pub project_id: String,
    pub provider: ProviderId,
    pub project_type: ProjectType,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub dependency: bool,
}

pub fn diff_locked(current: &[ContentItem], next: &[ContentItem]) -> Vec<ImpactChange> {
    let cur: HashMap<String, &ContentItem> = current.iter().map(|i| (i.project_id(), i)).collect();
    let nxt: HashMap<String, &ContentItem> = next.iter().map(|i| (i.project_id(), i)).collect();
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
                        key: crate::content::item_rel_path(ci),
                        project_id: id.clone(),
                        provider: ni.preferred,
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
                key: crate::content::item_rel_path(ni),
                project_id: id.clone(),
                provider: ni.preferred,
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
                key: crate::content::item_rel_path(ci),
                project_id: id.clone(),
                provider: ci.preferred,
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
    let present: HashSet<String> = out.locked.iter().map(|i| i.project_id()).collect();
    let mut v = Vec::new();
    for c in &out.conflicts {
        let versions: Vec<String> = c.picks.iter().map(|p| p.version.clone()).collect();
        v.push(format!(
            "{} requires conflicting versions: {}",
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
        if present.contains(&inc.project_id) && present.contains(&inc.with_project_id) {
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
    pub latest: Option<(String, String)>,
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

pub type Aliases = HashMap<(ProviderId, String), (ProviderId, String)>;

pub fn aliases(items: &[ContentItem]) -> Aliases {
    let in_use: HashSet<String> = items
        .iter()
        .filter_map(|i| i.active().and_then(|s| s.id.clone()))
        .collect();
    let mut map = Aliases::new();
    for item in items {
        let Some(main) = item
            .active()
            .and_then(|s| s.id.clone())
            .filter(|id| !id.is_empty())
        else {
            continue;
        };
        for (provider, source) in &item.sources {
            if *provider == item.preferred {
                continue;
            }
            let Some(id) = source.id.clone().filter(|id| !id.is_empty()) else {
                continue;
            };
            if !in_use.contains(&id) {
                map.insert((*provider, id), (item.preferred, main.clone()));
            }
        }
    }
    map
}

struct Target {
    provider: ProviderId,
    id: Option<String>,
    url: Option<String>,
    path: Option<String>,
    version: Option<String>,
    side: String,
    explicit: bool,
    required_by: Option<String>,
    project_type: ProjectType,
    name: String,
    file: String,
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

struct Ctx<'a> {
    mr: &'a Modrinth,
    cf: &'a CurseForge,
    mc: &'a str,
    loader: &'a str,
    channel: u8,
    excluded: &'a HashSet<String>,
    known: &'a HashMap<String, ContentItem>,
    aliases: &'a Aliases,
}

impl Ctx<'_> {
    fn canonical(&self, provider: ProviderId, id: &str) -> (ProviderId, String) {
        self.aliases
            .get(&(provider, id.to_string()))
            .cloned()
            .unwrap_or_else(|| (provider, id.to_string()))
    }

    fn loader_for(&self, project_type: ProjectType) -> &str {
        if project_type == ProjectType::Mod {
            self.loader
        } else {
            ""
        }
    }

    async fn fetch(
        &self,
        provider: ProviderId,
        id: &str,
        version: Option<&str>,
        project_type: ProjectType,
        name: &str,
    ) -> Result<Resolved> {
        match providers::by_id(provider, self.mr, self.cf) {
            Some(p) => {
                p.resolve(
                    id,
                    version,
                    self.mc,
                    self.loader_for(project_type),
                    project_type,
                    self.channel,
                )
                .await
            }
            None => Err(anyhow!(
                "Provider '{}' is not supported yet ({}).",
                provider,
                name
            )),
        }
    }
}

#[derive(Default)]
struct Crawl {
    chosen: HashMap<String, (Target, Resolved)>,
    leaves: Vec<ContentItem>,
    explicit_set: HashSet<String>,
    dependents: HashMap<String, HashSet<String>>,
    issues: Vec<ResolveIssue>,
    incompatible: Vec<Incompat>,
    optional: Vec<OptionalDep>,
    disabled_edges: Vec<(String, String, ProviderId)>,
    visited: HashSet<String>,
    alt_map: HashMap<String, Vec<(ProviderId, SourceFile)>>,
    version_requests: HashMap<String, BTreeMap<String, BTreeSet<String>>>,
    merged: HashMap<String, String>,
}

pub async fn resolve(
    mr: &Modrinth,
    cf: &CurseForge,
    manifest: &Manifest,
    authored: &[ContentItem],
    excluded: &HashSet<String>,
    known: &HashMap<String, ContentItem>,
    aliases: &Aliases,
) -> Result<ResolveOutput> {
    let ctx = Ctx {
        mr,
        cf,
        mc: &manifest.minecraft,
        loader: &manifest.loader,
        channel: manifest.channel.rank(),
        excluded,
        known,
        aliases,
    };

    let mut st = Crawl::default();
    let frontier = seed_frontier(&mut st, authored);
    crawl(&ctx, &mut st, frontier).await;

    collect_disabled_dependency_issues(&mut st);
    let conflicts = resolve_version_conflicts(&ctx, &st).await;
    let names = lookup_missing_names(&ctx, &st, &conflicts).await;

    let mut locked = build_locked(&st);
    backfill_alt_sources(&ctx, &st, &mut locked).await;
    locked.sort_by(|a, b| {
        b.explicit
            .cmp(&a.explicit)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(ResolveOutput {
        locked,
        issues: st.issues,
        incompatible: st.incompatible,
        optional: st.optional,
        conflicts,
        names,
    })
}

fn seed_frontier(st: &mut Crawl, authored: &[ContentItem]) -> Vec<Target> {
    let mut frontier: Vec<Target> = Vec::new();
    for item in authored {
        let pref = item.preferred;
        let psrc = item.sources.get(&pref).cloned().unwrap_or_default();
        let key = src_ident(&psrc).unwrap_or_else(|| item.name.clone());
        st.explicit_set.insert(key.clone());
        let alts: Vec<(ProviderId, SourceFile)> = item
            .sources
            .iter()
            .filter(|(p, _)| **p != pref)
            .map(|(p, s)| (*p, s.clone()))
            .collect();
        if !alts.is_empty() {
            st.alt_map.insert(key.clone(), alts);
        }
        frontier.push(Target {
            provider: pref,
            id: psrc.id.clone(),
            url: psrc.url.clone(),
            path: psrc.path.clone(),
            version: Some(psrc.version_id.clone()).filter(|v| !v.is_empty()),
            side: item.side.clone(),
            explicit: true,
            required_by: None,
            project_type: item.project_type,
            name: item.name.clone(),
            file: item.file.clone(),
        });
    }
    frontier
}

async fn crawl(ctx: &Ctx<'_>, st: &mut Crawl, mut frontier: Vec<Target>) {
    while !frontier.is_empty() {
        let layer = admit_layer(st, frontier.drain(..));
        if layer.is_empty() {
            break;
        }

        let api_targets = split_off_offline_leaves(st, layer);

        let fetches = api_targets.into_iter().map(|t| async move {
            let res = ctx
                .fetch(
                    t.provider,
                    t.id.as_deref().unwrap_or(""),
                    t.version.as_deref(),
                    t.project_type,
                    &t.name,
                )
                .await;
            (t, res)
        });
        let results: Vec<(Target, Result<Resolved>)> = futures::stream::iter(fetches)
            .buffer_unordered(FETCH_CONCURRENCY)
            .collect()
            .await;

        let mut next: Vec<Target> = Vec::new();
        for (target, res) in results {
            match res {
                Ok(r) => next.extend(record_resolved(ctx, st, target, r)),
                Err(e) => record_failure(ctx, st, target, e).await,
            }
        }

        frontier = next;
    }
}

fn admit_layer(st: &mut Crawl, targets: impl Iterator<Item = Target>) -> Vec<Target> {
    let mut layer: Vec<Target> = Vec::new();
    for t in targets {
        let k = t.key();
        if let Some(into) = st.merged.get(&k).cloned() {
            if let Some(parent) = &t.required_by {
                st.dependents
                    .entry(into)
                    .or_default()
                    .insert(parent.clone());
            }
            continue;
        }
        if let Some(parent) = &t.required_by {
            st.dependents
                .entry(k.clone())
                .or_default()
                .insert(parent.clone());
        }
        if let Some(want) = t.version.clone().filter(|p| !p.is_empty()) {
            let by = t.required_by.clone().unwrap_or_else(|| "pack".into());
            st.version_requests
                .entry(k.clone())
                .or_default()
                .entry(want)
                .or_default()
                .insert(by);
        }
        if st.visited.contains(&k) {
            continue;
        }
        st.visited.insert(k);
        layer.push(t);
    }
    layer
}

fn split_off_offline_leaves(st: &mut Crawl, layer: Vec<Target>) -> Vec<Target> {
    let mut api_targets: Vec<Target> = Vec::new();
    for t in layer {
        match t.provider {
            ProviderId::Url => st.leaves.push(leaf_from_url(&t)),
            ProviderId::Local => match leaf_from_local(&t) {
                Some(item) => st.leaves.push(item),
                None => st.issues.push(ResolveIssue {
                    kind: ISSUE_MISSING_FILE.into(),
                    project_id: t.key(),
                    provider: t.provider,
                    detail: String::new(),
                    required_by: t.required_by.iter().cloned().collect(),
                    latest: None,
                }),
            },
            _ => api_targets.push(t),
        }
    }
    api_targets
}

fn record_resolved(ctx: &Ctx<'_>, st: &mut Crawl, mut target: Target, r: Resolved) -> Vec<Target> {
    if let Some(actual) = r.actual_type {
        target.project_type = actual;
    }
    let key = target.key();
    if !target.explicit {
        if let Some(into) = same_file(st, &key, &r) {
            if let Some(parents) = st.dependents.remove(&key) {
                st.dependents
                    .entry(into.clone())
                    .or_default()
                    .extend(parents);
            }
            st.version_requests.remove(&key);
            st.merged.insert(key, into);
            return Vec::new();
        }
    }
    let mut next = Vec::new();
    for dep in &r.deps {
        let (provider, id) = ctx.canonical(dep.provider, &dep.id);
        let asked = dep.version.clone().filter(|_| provider == dep.provider);
        match dep.kind {
            DepKind::Required => {
                if ctx.excluded.contains(&id) {
                    st.disabled_edges.push((key.clone(), id, provider));
                } else {
                    let known = ctx.known.get(&id);
                    let held = known
                        .filter(|i| i.pinned)
                        .and_then(|i| i.sources.get(&i.preferred))
                        .map(|s| s.version_id.clone())
                        .filter(|v| !v.is_empty());
                    let want = match &held {
                        Some(pinned) => {
                            if let Some(asked) = asked.filter(|v| v != pinned) {
                                st.version_requests
                                    .entry(id.clone())
                                    .or_default()
                                    .entry(asked)
                                    .or_default()
                                    .insert(key.clone());
                            }
                            held.clone()
                        }
                        None => asked.or_else(|| {
                            known
                                .and_then(|i| i.sources.get(&i.preferred))
                                .map(|s| s.version_id.clone())
                                .filter(|v| !v.is_empty())
                        }),
                    };
                    let kind = known
                        .map(|i| i.project_type)
                        .unwrap_or(match target.project_type {
                            ProjectType::Datapack => ProjectType::Datapack,
                            _ => ProjectType::Mod,
                        });
                    let file = known.map(|i| i.file.clone()).unwrap_or_default();
                    next.push(dep_target(provider, id, want, key.clone(), kind, file));
                }
            }
            DepKind::Optional => st.optional.push(OptionalDep {
                project_id: id,
                provider,
                required_by: key.clone(),
            }),
            DepKind::Incompatible => st.incompatible.push(Incompat {
                project_id: key.clone(),
                provider: target.provider,
                with_project_id: id,
            }),
        }
    }
    st.chosen.insert(key, (target, r));
    next
}

fn same_file(st: &Crawl, key: &str, r: &Resolved) -> Option<String> {
    let sha1 = r.sha1.as_deref().filter(|s| !s.is_empty())?;
    st.chosen
        .iter()
        .find(|(k, (_, other))| k.as_str() != key && other.sha1.as_deref() == Some(sha1))
        .map(|(k, _)| k.clone())
}

async fn record_failure(ctx: &Ctx<'_>, st: &mut Crawl, target: Target, e: anyhow::Error) {
    let key = target.key();
    let mut required_by: Vec<String> = st
        .dependents
        .get(&key)
        .map(|s| s.iter().cloned().collect())
        .unwrap_or_default();
    required_by.sort();

    if target.version.is_some() {
        let retry = ctx
            .fetch(
                target.provider,
                target.id.as_deref().unwrap_or(""),
                None,
                target.project_type,
                &target.name,
            )
            .await
            .ok();
        if let Some(r) = retry {
            st.issues.push(ResolveIssue {
                kind: ISSUE_INVALID_VERSION.into(),
                project_id: key.clone(),
                provider: target.provider,
                detail: format!("{e}"),
                required_by,
                latest: Some((r.version_id.clone(), r.version_number.clone())),
            });
            if let Some(prev) = ctx.known.get(&key) {
                let mut item = prev.clone();
                item.disabled = false;
                st.leaves.push(item);
            } else if target.explicit {
                st.leaves.push(unresolved_leaf(&target));
            }
            return;
        }
    }

    let kind = if e.downcast_ref::<providers::NoVersion>().is_none() {
        ISSUE_PROVIDER
    } else if target.explicit {
        ISSUE_MISSING_VERSION
    } else {
        ISSUE_MISSING_DEPENDENCY
    };
    st.issues.push(ResolveIssue {
        kind: kind.into(),
        project_id: key,
        provider: target.provider,
        detail: format!("{e}"),
        required_by,
        latest: None,
    });
    if target.explicit {
        st.leaves.push(unresolved_leaf(&target));
    }
}

fn collect_disabled_dependency_issues(st: &mut Crawl) {
    let mut by_disabled: HashMap<(String, ProviderId), BTreeSet<String>> = HashMap::new();
    for (parent, dep_id, prov) in std::mem::take(&mut st.disabled_edges) {
        if st.chosen.contains_key(&dep_id) {
            continue;
        }
        by_disabled
            .entry((dep_id, prov))
            .or_default()
            .insert(parent);
    }
    for ((dep_id, prov), parents) in by_disabled {
        st.issues.push(ResolveIssue {
            kind: ISSUE_DISABLED_DEPENDENCY.into(),
            project_id: dep_id,
            provider: prov,
            detail: String::new(),
            required_by: parents.into_iter().collect(),
            latest: None,
        });
    }
}

async fn resolve_version_conflicts(ctx: &Ctx<'_>, st: &Crawl) -> Vec<VersionConflict> {
    let mut conflicts: Vec<VersionConflict> = Vec::new();
    for (key, wants) in &st.version_requests {
        if wants.len() < 2 {
            continue;
        }
        let (provider, ptype, win_version, win_num) = match st.chosen.get(key) {
            Some((t, r)) => (
                t.provider,
                t.project_type,
                t.version.clone(),
                r.version_number.clone(),
            ),
            None => (ProviderId::Modrinth, ProjectType::Mod, None, String::new()),
        };
        let pinned = ctx
            .known
            .get(key)
            .filter(|i| i.pinned)
            .and_then(|i| i.sources.get(&i.preferred))
            .map(|s| s.version_id.clone());
        let mut picks = Vec::new();
        let mut pinned_number = None;
        for (want, by) in wants {
            let version = if win_version.as_deref() == Some(want.as_str()) && !win_num.is_empty() {
                win_num.clone()
            } else {
                ctx.fetch(provider, key, Some(want), ptype, key)
                    .await
                    .map(|r| r.version_number)
                    .ok()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| want.clone())
            };
            let held = pinned.as_deref() == Some(want.as_str());
            if held {
                pinned_number = Some(version.clone());
            }
            picks.push(ConflictPick {
                version,
                requested_by: if held {
                    vec!["pack".into()]
                } else {
                    by.iter().cloned().collect()
                },
            });
        }
        picks.sort_by(|a, b| a.version.cmp(&b.version));
        conflicts.push(VersionConflict {
            project_id: key.clone(),
            provider,
            picks,
            pinned: pinned_number,
            current: win_num,
        });
    }
    conflicts.sort_by(|a, b| a.project_id.cmp(&b.project_id));
    conflicts
}

async fn lookup_missing_names(
    ctx: &Ctx<'_>,
    st: &Crawl,
    conflicts: &[VersionConflict],
) -> HashMap<String, ProjectMeta> {
    let mut meta: HashMap<String, ProjectMeta> = HashMap::new();
    for (key, (_t, r)) in &st.chosen {
        meta.insert(
            key.clone(),
            ProjectMeta {
                name: r.name.clone(),
                slug: r.slug.clone(),
            },
        );
    }

    let mut need: HashMap<ProviderId, HashSet<String>> = HashMap::new();
    let mut want = |prov: ProviderId, id: &String| {
        if !meta.contains_key(id) {
            need.entry(prov).or_default().insert(id.clone());
        }
    };
    for o in &st.optional {
        want(o.provider, &o.project_id);
    }
    for i in &st.incompatible {
        want(i.provider, &i.with_project_id);
    }
    for iss in &st.issues {
        want(iss.provider, &iss.project_id);
    }
    for c in conflicts {
        want(c.provider, &c.project_id);
    }

    for (prov, ids) in need {
        if let Some(p) = providers::by_id(prov, ctx.mr, ctx.cf) {
            let ids: Vec<String> = ids.into_iter().collect();
            for (id, (name, slug)) in p.names(&ids).await {
                meta.entry(id).or_insert(ProjectMeta { name, slug });
            }
        }
    }
    meta
}

fn optional_for(st: &Crawl, key: &str) -> Vec<String> {
    let mut wanted: Vec<String> = st
        .optional
        .iter()
        .filter(|o| o.project_id == key)
        .map(|o| o.required_by.clone())
        .collect();
    wanted.sort();
    wanted.dedup();
    wanted
}

fn build_locked(st: &Crawl) -> Vec<ContentItem> {
    let mut locked: Vec<ContentItem> = st.leaves.clone();
    for (key, (target, r)) in &st.chosen {
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
        let mut deps: Vec<String> = st
            .dependents
            .get(key)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        deps.sort();

        let mut sources = BTreeMap::new();
        sources.insert(target.provider, source_file(target, r));
        let keep_name = st.alt_map.contains_key(key) && !target.name.is_empty();
        locked.push(ContentItem {
            name: if keep_name || r.name.is_empty() {
                target.name.clone()
            } else {
                r.name.clone()
            },
            project_type: target.project_type,
            file: target.file.clone(),
            side: target.side.clone(),
            explicit: st.explicit_set.contains(key),
            disabled: false,
            pinned: false,
            preferred: target.provider,
            sources,
            dependents: deps,
            optional_dependents: optional_for(st, key),
            client_side,
            server_side,
            no_bundle: false,
        });
    }
    locked
}

async fn matching_version(
    ctx: &Ctx<'_>,
    provider: ProviderId,
    id: &str,
    item: &ContentItem,
) -> Option<String> {
    let number = clean_version(&item.active()?.version_number);
    if number.is_empty() || id.is_empty() {
        return None;
    }
    let source = providers::by_id(provider, ctx.mr, ctx.cf)?;
    let loader = item.project_type.version_loader(ctx.loader);
    let list = source.versions(id, ctx.mc, loader).await.ok()?;
    list.iter()
        .find(|v| clean_version(&v.version_number) == number)
        .map(|v| v.id.clone())
}

fn clean_version(number: &str) -> String {
    number.trim().trim_start_matches('v').to_lowercase()
}

const FINGERPRINT_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Default)]
struct SameFiles {
    modrinth: HashMap<String, (String, String)>,
    curseforge: HashMap<String, (String, String)>,
}

impl SameFiles {
    fn version(&self, provider: ProviderId, id: &str, item: &ContentItem) -> Option<String> {
        let sha1 = item.active()?.sha1.as_deref()?;
        let found = match provider {
            ProviderId::Modrinth => self.modrinth.get(sha1),
            ProviderId::Curseforge => self.curseforge.get(sha1),
            _ => None,
        }?;
        (found.0 == id).then(|| found.1.clone())
    }
}

async fn same_files(ctx: &Ctx<'_>, st: &Crawl, locked: &[ContentItem]) -> SameFiles {
    let mut modrinth: Vec<String> = Vec::new();
    let mut curseforge: Vec<(String, String)> = Vec::new();
    for item in locked.iter().filter(|i| i.explicit) {
        let Some(active) = item.active() else {
            continue;
        };
        let Some(sha1) = active.sha1.clone().filter(|s| !s.is_empty()) else {
            continue;
        };
        let key = src_ident(active).unwrap_or_else(|| item.name.clone());
        let Some(alts) = st.alt_map.get(&key) else {
            continue;
        };
        for (provider, src) in alts {
            if !src.version_id.is_empty() || item.sources.contains_key(provider) {
                continue;
            }
            match provider {
                ProviderId::Modrinth => modrinth.push(sha1.clone()),
                ProviderId::Curseforge
                    if CurseForge::configured()
                        && !active.download_url.is_empty()
                        && active.file_size <= FINGERPRINT_LIMIT =>
                {
                    curseforge.push((sha1.clone(), active.download_url.clone()))
                }
                _ => {}
            }
        }
    }
    let mut out = SameFiles::default();
    if !modrinth.is_empty() {
        if let Ok(found) = ctx.mr.versions_by_hashes(&modrinth).await {
            for (sha1, v) in found {
                out.modrinth.insert(sha1, (v.project_id, v.id));
            }
        }
    }
    let mut prints: Vec<u32> = Vec::new();
    let mut by_print: HashMap<u64, String> = HashMap::new();
    for (sha1, url) in curseforge {
        if let Some(print) = ctx.mr.curseforge_fingerprint(&sha1, &url).await {
            prints.push(print);
            by_print.insert(u64::from(print), sha1);
        }
    }
    if let Ok(found) = ctx.cf.fingerprints(&prints).await {
        for (print, (mod_id, file)) in found {
            if let Some(sha1) = by_print.get(&print) {
                out.curseforge
                    .insert(sha1.clone(), (mod_id.to_string(), file.id.to_string()));
            }
        }
    }
    out
}

async fn backfill_alt_sources(ctx: &Ctx<'_>, st: &Crawl, locked: &mut [ContentItem]) {
    let same = same_files(ctx, st, locked).await;
    for item in locked.iter_mut() {
        if !item.explicit {
            continue;
        }
        let key = item
            .sources
            .get(&item.preferred)
            .and_then(src_ident)
            .unwrap_or_else(|| item.name.clone());
        let alts = match st.alt_map.get(&key) {
            Some(a) => a.clone(),
            None => continue,
        };
        for (provider, src) in alts {
            if item.sources.contains_key(&provider) {
                continue;
            }
            let id = src.id.as_deref().unwrap_or("");
            let wanted = if src.version_id.is_empty() {
                match same.version(provider, id, item) {
                    Some(v) => Some(v),
                    None => matching_version(ctx, provider, id, item).await,
                }
            } else {
                Some(src.version_id.clone())
            };
            let Some(wanted) = wanted else {
                let slug = if src.slug.is_empty() {
                    ctx.fetch(provider, id, None, item.project_type, &item.name)
                        .await
                        .map(|r| r.slug)
                        .unwrap_or_default()
                } else {
                    src.slug.clone()
                };
                item.sources.insert(
                    provider,
                    SourceFile {
                        id: src.id.clone(),
                        url: src.url.clone(),
                        path: src.path.clone(),
                        slug,
                        pinned: src.pinned,
                        ..Default::default()
                    },
                );
                continue;
            };
            let resolved = ctx
                .fetch(provider, id, Some(&wanted), item.project_type, &item.name)
                .await
                .ok();
            let entry = match resolved {
                Some(r) => SourceFile {
                    id: src.id.clone(),
                    url: src.url.clone(),
                    path: src.path.clone(),
                    slug: r.slug,
                    version_id: r.version_id,
                    version_number: r.version_number,
                    filename: r.filename,
                    download_url: r.download_url,
                    sha1: r.sha1,
                    sha512: r.sha512,
                    file_size: r.file_size,
                    pinned: src.pinned,
                },
                None => SourceFile {
                    id: src.id.clone(),
                    url: src.url.clone(),
                    path: src.path.clone(),
                    pinned: src.pinned,
                    ..Default::default()
                },
            };
            item.sources.insert(provider, entry);
        }
    }
}

fn dep_target(
    provider: ProviderId,
    id: String,
    version: Option<String>,
    parent: String,
    project_type: ProjectType,
    file: String,
) -> Target {
    Target {
        provider,
        id: Some(id),
        url: None,
        path: None,
        version,
        side: "auto".into(),
        explicit: false,
        required_by: Some(parent),
        project_type,
        name: String::new(),
        file,
    }
}

fn source_file(t: &Target, r: &Resolved) -> SourceFile {
    SourceFile {
        id: t.id.clone(),
        url: t.url.clone(),
        path: t.path.clone(),
        slug: r.slug.clone(),
        version_id: r.version_id.clone(),
        version_number: r.version_number.clone(),
        filename: r.filename.clone(),
        download_url: r.download_url.clone(),
        sha1: r.sha1.clone(),
        sha512: r.sha512.clone(),
        file_size: r.file_size,
        pinned: false,
    }
}

fn sides_for(side: &str) -> (String, String) {
    match side {
        "client" => ("required".into(), "unsupported".into()),
        "server" => ("unsupported".into(), "required".into()),
        _ => ("required".into(), "required".into()),
    }
}

fn base_item(t: &Target, sources: BTreeMap<ProviderId, SourceFile>) -> ContentItem {
    let (client_side, server_side) = sides_for(&t.side);
    ContentItem {
        name: t.name.clone(),
        project_type: t.project_type,
        file: t.file.clone(),
        side: t.side.clone(),
        explicit: t.explicit,
        disabled: false,
        pinned: false,
        preferred: t.provider,
        sources,
        dependents: t.required_by.iter().cloned().collect(),
        optional_dependents: Vec::new(),
        no_bundle: false,
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
    let actual = providers::file_type(t.project_type, &filename, &[]);
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
    let mut item = base_item(t, sources);
    if let Some(actual) = actual {
        item.project_type = actual;
    }
    item
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
            version_id: t.version.clone().unwrap_or_default(),
            ..Default::default()
        },
    );
    base_item(t, sources)
}
