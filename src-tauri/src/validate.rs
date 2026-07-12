use serde::Serialize;
use std::collections::HashSet;

use crate::lockfile::{DepType, Lockfile};
use crate::resolver::{
    ResolveOutput, ISSUE_MISSING_DEPENDENCY, ISSUE_MISSING_VERSION,
};

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Validation {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub project_id: Option<String>,
    pub fix: Option<Fix>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fix {
    pub kind: String,
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub label: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub ok: bool,
    pub errors: u32,
    pub warnings: u32,
    pub infos: u32,
    pub mod_count: u32,
    pub dependency_count: u32,
}

pub fn build(
    out: &ResolveOutput,
    lock: &Lockfile,
) -> (Vec<Validation>, Health) {
    let present: HashSet<&str> =
        lock.mods.iter().map(|m| m.project_id.as_str()).collect();
    let name_of = |id: &str| -> String {
        out.names
            .get(id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let slug_of = |id: &str| -> String {
        out.names
            .get(id)
            .map(|m| m.slug.clone())
            .unwrap_or_else(|| id.to_string())
    };

    let mut items: Vec<Validation> = Vec::new();
    let mut idx = 0usize;

    let env = format!("{} {}", lock.minecraft, lock.loader);
    for issue in &out.issues {
        let title;
        let detail;
        let fix;
        if issue.kind == ISSUE_MISSING_VERSION {
            title =
                format!("{}: no compatible build", name_of(&issue.project_id));
            detail = format!("No version for {env}.");
            fix = Some(Fix {
                kind: "remove".into(),
                project_id: issue.project_id.clone(),
                slug: slug_of(&issue.project_id),
                name: name_of(&issue.project_id),
                label: "Remove it".into(),
            });
        } else if issue.kind == ISSUE_MISSING_DEPENDENCY {
            title =
                format!("Missing dependency: {}", name_of(&issue.project_id));
            detail = if issue.required_by.is_empty() {
                format!("Required, but has no build for {env}.")
            } else {
                let names: Vec<String> =
                    issue.required_by.iter().map(|id| name_of(id)).collect();
                format!("Needed by {}, no build for {env}.", join_names(&names))
            };
            fix = None;
        } else {
            title = format!("Couldn’t load {}", name_of(&issue.project_id));
            detail = issue.detail.clone();
            fix = None;
        }
        items.push(Validation {
            id: format!("issue-{idx}"),
            severity: "error".into(),
            title,
            detail,
            project_id: Some(issue.project_id.clone()),
            fix,
        });
        idx += 1;
    }

    for c in &out.conflicts {
        let parts: Vec<String> = c
            .picks
            .iter()
            .map(|p| {
                let mut who: Vec<String> = p
                    .requested_by
                    .iter()
                    .map(|id| {
                        if id == "pack" {
                            "you".to_string()
                        } else {
                            name_of(id)
                        }
                    })
                    .collect();
                who.sort();
                who.dedup();
                format!("{} ({})", p.version, join_names(&who))
            })
            .collect();
        items.push(Validation {
            id: format!("conflict-{idx}"),
            severity: "error".into(),
            title: format!("{}: version conflict", name_of(&c.project_id)),
            detail: format!(
                "Required at {}. Only one can install. Pin the one to keep.",
                parts.join(", ")
            ),
            project_id: Some(c.project_id.clone()),
            fix: None,
        });
        idx += 1;
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
        let key = (pair[0].clone(), pair[1].clone());
        if !seen_pairs.insert(key) {
            continue;
        }
        items.push(Validation {
            id: format!("incompat-{idx}"),
            severity: "warning".into(),
            title: format!(
                "{} & {} can’t run together",
                name_of(&inc.project_id),
                name_of(&inc.with_project_id)
            ),
            detail: "Marked incompatible by their authors. Remove one.".into(),
            project_id: Some(inc.with_project_id.clone()),
            fix: Some(Fix {
                kind: "remove".into(),
                project_id: inc.with_project_id.clone(),
                slug: slug_of(&inc.with_project_id),
                name: name_of(&inc.with_project_id),
                label: format!("Remove {}", name_of(&inc.with_project_id)),
            }),
        });
        idx += 1;
    }

    let mut seen_opt: HashSet<String> = HashSet::new();
    for opt in &out.optional {
        if present.contains(opt.project_id.as_str()) {
            continue;
        }
        if !seen_opt.insert(opt.project_id.clone()) {
            continue;
        }
        items.push(Validation {
            id: format!("opt-{idx}"),
            severity: "info".into(),
            title: format!(
                "{} pairs well with {}",
                name_of(&opt.required_by),
                name_of(&opt.project_id)
            ),
            detail: "Add it if you want the extra features.".into(),
            project_id: Some(opt.project_id.clone()),
            fix: Some(Fix {
                kind: "add".into(),
                project_id: opt.project_id.clone(),
                slug: slug_of(&opt.project_id),
                name: name_of(&opt.project_id),
                label: format!("Add {}", name_of(&opt.project_id)),
            }),
        });
        idx += 1;
    }

    let errors = items.iter().filter(|v| v.severity == "error").count() as u32;
    let warnings =
        items.iter().filter(|v| v.severity == "warning").count() as u32;
    let infos = items.iter().filter(|v| v.severity == "info").count() as u32;

    let mod_count = lock
        .mods
        .iter()
        .filter(|m| m.dependency_type == DepType::Explicit)
        .count() as u32;
    let dependency_count = lock.mods.len() as u32 - mod_count;

    if errors == 0 && warnings == 0 {
        items.push(Validation {
            id: "ok".into(),
            severity: "ok".into(),
            title: "Everything checks out".into(),
            detail: format!(
                "{mod_count} mods plus {dependency_count} dependencies, all compatible with this Minecraft version and loader."
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
