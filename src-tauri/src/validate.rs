use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

use crate::lockfile::{DepType, Lockfile};
use crate::providers::{self, ProviderId};
use crate::ptype::ProjectType;
use crate::resolver::{
    ResolveOutput, ISSUE_DISABLED_DEPENDENCY, ISSUE_INVALID_VERSION, ISSUE_MISSING_DEPENDENCY,
    ISSUE_MISSING_FILE, ISSUE_MISSING_VERSION,
};
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Validation {
    pub id: String,
    pub key: String,
    #[ts(type = "'error' | 'warning' | 'info' | 'ok'")]
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub project_id: Option<String>,
    pub fix: Option<Fix>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Fix {
    #[ts(type = "'remove' | 'update' | 'enable' | 'unpin'")]
    pub kind: String,
    pub project_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Health {
    pub ok: bool,
    pub errors: u32,
    pub warnings: u32,
    pub infos: u32,
    pub mod_count: u32,
    pub dependency_count: u32,
}

pub struct LocalMod {
    stem: String,
    filename: String,
}

fn stem(text: &str) -> String {
    let normal = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    match normal.strip_suffix("api") {
        Some(rest) if rest.len() >= 3 => rest.to_string(),
        _ => normal,
    }
}

pub fn local_mods(pack_dir: &Path) -> Vec<LocalMod> {
    let Ok(entries) = std::fs::read_dir(crate::instance::overrides_dir(pack_dir).join("mods"))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jar") {
            continue;
        }
        let Some(meta) = crate::modmeta::read_archive(&path) else {
            continue;
        };
        let filename = entry.file_name().to_string_lossy().to_string();
        for id in &meta.ids {
            out.push(LocalMod {
                stem: stem(id),
                filename: filename.clone(),
            });
        }
    }
    out
}

fn provided_by<'a>(local: &'a [LocalMod], slug: &str, name: &str) -> Option<&'a str> {
    let wanted = [stem(slug), stem(name)];
    local
        .iter()
        .find(|m| wanted.iter().any(|w| w.len() >= 3 && *w == m.stem))
        .map(|m| m.filename.as_str())
}

fn join_names(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} and {b}"),
        _ => {
            let (last, rest) = names.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

struct Names<'a> {
    out: &'a ResolveOutput,
}

impl Names<'_> {
    fn name(&self, id: &str) -> String {
        self.out
            .names
            .get(id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    fn slug(&self, id: &str) -> String {
        self.out
            .names
            .get(id)
            .map(|m| m.slug.clone())
            .unwrap_or_else(|| id.to_string())
    }

    fn all(&self, ids: &[String]) -> String {
        let names: Vec<String> = ids
            .iter()
            .map(|id| {
                if id == "pack" {
                    "you".to_string()
                } else {
                    self.name(id)
                }
            })
            .collect();
        join_names(&names)
    }

    fn who(&self, ids: &[String], one: &str, many: &str) -> String {
        let plural = ids.len() > 1 || ids.iter().any(|id| id == "pack");
        format!("{} {}", self.all(ids), if plural { many } else { one })
    }

    fn fix(&self, kind: &str, id: &str, label: String) -> Fix {
        Fix {
            kind: kind.into(),
            project_id: id.to_string(),
            label,
            version: String::new(),
        }
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn plural(count: u32, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn game(lock: &Lockfile, project_type: ProjectType) -> String {
    if project_type == ProjectType::Mod && lock.loader != "vanilla" {
        format!(
            "{} {}",
            lock.minecraft,
            providers::loader_name(&lock.loader)
        )
    } else {
        lock.minecraft.clone()
    }
}

pub(crate) fn reason(detail: &str, provider: ProviderId) -> (String, bool) {
    let text = detail.to_lowercase();
    let host = provider.label();
    let has = |keys: &[&str]| keys.iter().any(|k| text.contains(k));
    if has(&[
        "error sending request",
        "dns error",
        "timed out",
        "connection refused",
        "connection reset",
    ]) {
        (format!("Couldn't reach {host}"), true)
    } else if has(&["429"]) {
        (format!("{host} is limiting requests"), true)
    } else if has(&["server error", "error 5"]) {
        (format!("{host} is having trouble"), true)
    } else if has(&["404", "not found"]) {
        (format!("Not found on {host}"), false)
    } else {
        let first = detail.lines().next().unwrap_or("").trim();
        (first.trim_end_matches('.').to_string(), false)
    }
}

fn some_names(names: &[String]) -> String {
    if names.len() <= 3 {
        return join_names(names);
    }
    format!("{} and {} more", names[..3].join(", "), names.len() - 3)
}

pub fn build(
    out: &ResolveOutput,
    lock: &Lockfile,
    local: &[LocalMod],
) -> (Vec<Validation>, Health) {
    let present: HashSet<&str> = lock.mods.iter().map(|m| m.project_id.as_str()).collect();
    let names = Names { out };
    let type_of = |id: &str| {
        lock.mods
            .iter()
            .find(|m| m.project_id == id)
            .map(|m| m.project_type)
            .unwrap_or(ProjectType::Mod)
    };
    let mut items: Vec<Validation> = Vec::new();
    let mut push = |severity: &str,
                    key: String,
                    title: String,
                    detail: String,
                    id: Option<&str>,
                    fix: Option<Fix>| {
        items.push(Validation {
            id: format!("problem-{}", items.len()),
            key,
            severity: severity.into(),
            title,
            detail,
            project_id: id.map(str::to_string),
            fix,
        });
    };
    let mut unreachable: Vec<(String, String, String, bool, ProviderId)> = Vec::new();

    for issue in &out.issues {
        let id = issue.project_id.as_str();
        let name = names.name(id);
        let target = game(lock, type_of(id));
        if issue.kind == ISSUE_MISSING_VERSION {
            push(
                "error",
                format!("missing_version:{id}"),
                format!("{name} has no version for {target}"),
                String::new(),
                Some(id),
                Some(names.fix("remove", id, "Remove".into())),
            );
        } else if issue.kind == ISSUE_INVALID_VERSION {
            let (latest_id, latest_number) = issue.latest.clone().unwrap_or_default();
            let mut fix = names.fix(
                "update",
                id,
                if latest_number.is_empty() {
                    "Update".into()
                } else {
                    format!("Update to {latest_number}")
                },
            );
            fix.version = latest_id;
            push(
                "warning",
                format!("invalid_version:{id}"),
                format!(
                    "{name}'s version was removed from {}",
                    issue.provider.label()
                ),
                String::new(),
                Some(id),
                Some(fix),
            );
        } else if issue.kind == ISSUE_MISSING_DEPENDENCY {
            let title = if issue.required_by.is_empty() {
                format!("{name} has no version for {target}")
            } else {
                format!(
                    "{} {name}, which has no version for {target}",
                    names.who(&issue.required_by, "requires", "require")
                )
            };
            let key = format!("missing_dependency:{id}");
            match provided_by(local, &names.slug(id), &name) {
                Some(file) => push(
                    "warning",
                    key,
                    title,
                    format!("Provided by {file} in overrides"),
                    Some(id),
                    None,
                ),
                None => push("error", key, title, String::new(), Some(id), None),
            }
        } else if issue.kind == ISSUE_DISABLED_DEPENDENCY {
            let title = if issue.required_by.is_empty() {
                format!("{name} is required but disabled")
            } else {
                format!(
                    "{} {name}, which is disabled",
                    names.who(&issue.required_by, "requires", "require")
                )
            };
            push(
                "warning",
                format!("disabled_dependency:{id}"),
                title,
                String::new(),
                Some(id),
                Some(names.fix("enable", id, "Enable".into())),
            );
        } else if issue.kind == ISSUE_MISSING_FILE {
            push(
                "error",
                format!("missing_file:{id}"),
                format!("{name}'s file is missing"),
                String::new(),
                Some(id),
                Some(names.fix("remove", id, "Remove".into())),
            );
        } else {
            let (why, passing) = reason(&issue.detail, issue.provider);
            unreachable.push((id.to_string(), name, why, passing, issue.provider));
        }
    }

    let mut reasons: Vec<&str> = Vec::new();
    for (_, _, why, _, _) in &unreachable {
        if !reasons.contains(&why.as_str()) {
            reasons.push(why);
        }
    }
    for why in reasons {
        let group: Vec<&(String, String, String, bool, ProviderId)> =
            unreachable.iter().filter(|u| u.2 == why).collect();
        let severity = if group[0].3 { "warning" } else { "error" };
        if let [(id, name, _, _, _)] = group.as_slice() {
            push(
                severity,
                format!("load:{id}"),
                format!("Couldn't load {name}"),
                why.to_string(),
                Some(id),
                None,
            );
        } else {
            let listed: Vec<String> = group.iter().map(|u| u.1.clone()).collect();
            push(
                severity,
                format!("load:{why}"),
                why.to_string(),
                some_names(&listed),
                None,
                None,
            );
        }
    }

    for conflict in &out.conflicts {
        let project = names.name(&conflict.project_id);
        if let Some(pinned) = &conflict.pinned {
            let wants: Vec<String> = conflict
                .picks
                .iter()
                .filter(|p| &p.version != pinned)
                .map(|p| {
                    format!(
                        "{} {}",
                        names.who(&p.requested_by, "requires", "require"),
                        p.version
                    )
                })
                .collect();
            let fix = names.fix("unpin", &conflict.project_id, "Unpin".into());
            push(
                "warning",
                format!("pinned:{}", conflict.project_id),
                format!("{project} is pinned to {pinned}"),
                capitalize(&wants.join(", ")),
                Some(&conflict.project_id),
                Some(fix),
            );
            continue;
        }
        let mut wants: Vec<String> = conflict
            .picks
            .iter()
            .filter(|p| p.version != conflict.current)
            .map(|p| {
                format!(
                    "{} {}",
                    names.who(&p.requested_by, "requires", "require"),
                    p.version
                )
            })
            .collect();
        if !conflict.current.is_empty() {
            wants.push(format!("the current version is {}", conflict.current));
        }
        push(
            "error",
            format!("conflict:{}", conflict.project_id),
            format!("Conflicting versions of {project}"),
            capitalize(&wants.join(", ")),
            Some(&conflict.project_id),
            None,
        );
    }

    let mut seen_pairs: HashSet<(String, String)> = HashSet::new();
    for inc in &out.incompatible {
        if !present.contains(inc.project_id.as_str())
            || !present.contains(inc.with_project_id.as_str())
        {
            continue;
        }
        let mut pair = [inc.project_id.clone(), inc.with_project_id.clone()];
        pair.sort();
        if !seen_pairs.insert((pair[0].clone(), pair[1].clone())) {
            continue;
        }
        let other = names.name(&inc.with_project_id);
        push(
            "warning",
            format!("incompatible:{}:{}", pair[0], pair[1]),
            format!(
                "{} is incompatible with {other}",
                names.name(&inc.project_id)
            ),
            String::new(),
            Some(&inc.with_project_id),
            Some(names.fix("remove", &inc.with_project_id, format!("Remove {other}"))),
        );
    }

    let count = |severity: &str| items.iter().filter(|v| v.severity == severity).count() as u32;
    let errors = count("error");
    let warnings = count("warning");
    let infos = count("info");
    let mod_count = lock
        .mods
        .iter()
        .filter(|m| m.dependency_type == DepType::Explicit)
        .count() as u32;
    let dependency_count = lock.mods.len() as u32 - mod_count;

    if errors == 0 && warnings == 0 {
        items.push(Validation {
            id: "ok".into(),
            key: "ok".into(),
            severity: "ok".into(),
            title: "Everything checks out".into(),
            detail: format!(
                "{} and {}",
                plural(mod_count, "mod", "mods"),
                plural(dependency_count, "dependency", "dependencies")
            ),
            project_id: None,
            fix: None,
        });
    }

    let health = Health {
        ok: errors == 0,
        errors,
        warnings,
        infos,
        mod_count,
        dependency_count,
    };
    (items, health)
}
