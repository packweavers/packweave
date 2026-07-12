use std::path::PathBuf;

use crate::changelog;
use crate::git::{self, GitCommit, GitStatus};
use crate::github;
use crate::packdiff;
use crate::secrets;

use super::es;

#[tauri::command]
pub async fn git_status(path: String) -> Result<GitStatus, String> {
    git::status(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn git_init(path: String) -> Result<(), String> {
    git::init(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn git_commit(
    path: String,
    message: String,
    files: Vec<String>,
    amend: bool,
) -> Result<(), String> {
    git::commit(&PathBuf::from(path), &message, &files, amend).map_err(es)
}

#[tauri::command]
pub async fn git_log(
    path: String,
    limit: u32,
    refname: Option<String>,
    file: Option<String>,
) -> Result<Vec<GitCommit>, String> {
    git::log(
        &PathBuf::from(path),
        limit,
        refname.as_deref(),
        file.as_deref(),
    )
    .map_err(es)
}

#[tauri::command]
pub async fn git_push(
    path: String,
    force: bool,
    tags: bool,
) -> Result<String, String> {
    git::push(&PathBuf::from(path), force, tags).map_err(es)
}

#[tauri::command]
pub async fn git_pull(
    path: String,
    strategy: String,
) -> Result<String, String> {
    git::pull(&PathBuf::from(path), &strategy).map_err(es)
}

#[tauri::command]
pub async fn git_discard(path: String) -> Result<(), String> {
    git::discard_all(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn git_diff_file(
    path: String,
    file: String,
    staged: bool,
) -> Result<String, String> {
    git::diff_file(&PathBuf::from(path), &file, staged).map_err(es)
}

#[tauri::command]
pub async fn git_discard_file(
    path: String,
    file: String,
) -> Result<(), String> {
    git::discard_file(&PathBuf::from(path), &file).map_err(es)
}

#[tauri::command]
pub async fn git_revert(
    path: String,
    files: Vec<String>,
) -> Result<(), String> {
    git::revert_files(&PathBuf::from(path), &files).map_err(es)
}

#[tauri::command]
pub async fn git_resolve_conflict(
    path: String,
    file: String,
    side: String,
) -> Result<(), String> {
    git::resolve_conflict(&PathBuf::from(path), &file, &side).map_err(es)
}

#[tauri::command]
pub async fn git_branches(path: String) -> Result<git::Branches, String> {
    git::branches(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn git_checkout(path: String, branch: String) -> Result<(), String> {
    git::checkout(&PathBuf::from(path), &branch).map_err(es)
}

#[tauri::command]
pub async fn git_create_branch(
    path: String,
    name: String,
    start_point: Option<String>,
    checkout: bool,
) -> Result<(), String> {
    git::create_branch(
        &PathBuf::from(path),
        &name,
        start_point.as_deref(),
        checkout,
    )
    .map_err(es)
}

#[tauri::command]
pub async fn git_rename_branch(
    path: String,
    old: String,
    new: String,
) -> Result<(), String> {
    git::rename_branch(&PathBuf::from(path), &old, &new).map_err(es)
}

#[tauri::command]
pub async fn git_delete_branch(
    path: String,
    name: String,
    force: bool,
) -> Result<(), String> {
    git::delete_branch(&PathBuf::from(path), &name, force).map_err(es)
}

#[tauri::command]
pub async fn git_delete_remote_branch(
    path: String,
    name: String,
) -> Result<String, String> {
    git::delete_remote_branch(&PathBuf::from(path), &name).map_err(es)
}

#[tauri::command]
pub async fn git_merge(path: String, name: String) -> Result<String, String> {
    git::merge_branch(&PathBuf::from(path), &name).map_err(es)
}

#[tauri::command]
pub async fn git_rebase(path: String, name: String) -> Result<String, String> {
    git::rebase_branch(&PathBuf::from(path), &name).map_err(es)
}

#[tauri::command]
pub async fn git_set_upstream(
    path: String,
    upstream: String,
) -> Result<(), String> {
    git::set_upstream(&PathBuf::from(path), &upstream).map_err(es)
}

#[tauri::command]
pub async fn git_fetch(path: String) -> Result<String, String> {
    git::fetch(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn git_push_branch(
    path: String,
    remote: String,
    branch: String,
    set_upstream: bool,
) -> Result<String, String> {
    git::push_branch(&PathBuf::from(path), &remote, &branch, set_upstream)
        .map_err(es)
}

#[tauri::command]
pub async fn git_commit_changes(
    path: String,
    hash: String,
) -> Result<Vec<git::GitChange>, String> {
    git::commit_changes(&PathBuf::from(path), &hash).map_err(es)
}

#[tauri::command]
pub async fn git_show_diff(
    path: String,
    hash: String,
    file: String,
) -> Result<String, String> {
    git::show_diff(&PathBuf::from(path), &hash, &file).map_err(es)
}

#[tauri::command]
pub async fn git_pack_diff(
    path: String,
    from: String,
    to: String,
) -> Result<packdiff::PackDiff, String> {
    packdiff::pack_diff(&PathBuf::from(path), &from, &to).map_err(es)
}

#[tauri::command]
pub async fn git_pack_diff_working(
    path: String,
) -> Result<packdiff::PackDiff, String> {
    packdiff::pack_diff_working(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn changelog_between(
    path: String,
    from: String,
    to: String,
    heading: Option<String>,
    links: bool,
    for_file: bool,
) -> Result<String, String> {
    let diff =
        packdiff::pack_diff(&PathBuf::from(&path), &from, &to).map_err(es)?;
    Ok(changelog::render(
        &diff,
        heading.as_deref(),
        links,
        for_file,
    ))
}

#[tauri::command]
pub async fn changelog_working(
    path: String,
    heading: Option<String>,
    links: bool,
    for_file: bool,
) -> Result<String, String> {
    let diff =
        packdiff::pack_diff_working(&PathBuf::from(&path)).map_err(es)?;
    Ok(changelog::render(
        &diff,
        heading.as_deref(),
        links,
        for_file,
    ))
}

#[tauri::command]
pub async fn git_latest_tag(path: String) -> Result<Option<String>, String> {
    Ok(git::latest_tag(&PathBuf::from(path)))
}

const CHANGELOG_HEADER: &str = "# Changelog\n\n";

#[tauri::command]
pub async fn changelog_save(
    path: String,
    section: String,
) -> Result<(), String> {
    let file = PathBuf::from(path).join("CHANGELOG.md");
    let existing = std::fs::read_to_string(&file).unwrap_or_default();
    let body = existing
        .strip_prefix(CHANGELOG_HEADER)
        .unwrap_or(&existing)
        .trim_start()
        .to_string();
    let section = section.trim().to_string();

    let mut rest: &str = &body;
    if let Some(h) = section.lines().next().filter(|l| l.starts_with("## ")) {
        if let Some(stripped) = body.strip_prefix(h) {
            rest = match stripped.find("\n## ") {
                Some(pos) => &stripped[pos + 1..],
                None => "",
            };
        }
    }
    let rest = rest.trim();
    let out = if rest.is_empty() {
        format!("{CHANGELOG_HEADER}{section}\n")
    } else {
        format!("{CHANGELOG_HEADER}{section}\n\n{rest}\n")
    };
    std::fs::write(&file, out).map_err(es)
}

#[tauri::command]
pub async fn changelog_head(path: String) -> Result<Option<String>, String> {
    let file = PathBuf::from(path).join("CHANGELOG.md");
    let existing = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    let body = existing
        .strip_prefix(CHANGELOG_HEADER)
        .unwrap_or(&existing)
        .trim_start();
    let after_heading = match body.strip_prefix("## ") {
        Some(rest) => match rest.find('\n') {
            Some(nl) => &rest[nl + 1..],
            None => "",
        },
        None => body,
    };
    let section = match after_heading.find("\n## ") {
        Some(pos) => &after_heading[..pos],
        None => after_heading,
    }
    .trim();
    Ok(if section.is_empty() {
        None
    } else {
        Some(section.to_string())
    })
}

#[tauri::command]
pub async fn git_revert_commit(
    path: String,
    hash: String,
) -> Result<String, String> {
    git::revert_commit(&PathBuf::from(path), &hash).map_err(es)
}

#[tauri::command]
pub async fn git_reset(
    path: String,
    hash: String,
    mode: String,
) -> Result<(), String> {
    git::reset(&PathBuf::from(path), &hash, &mode).map_err(es)
}

#[tauri::command]
pub async fn git_cherry_pick(
    path: String,
    hash: String,
) -> Result<String, String> {
    git::cherry_pick(&PathBuf::from(path), &hash).map_err(es)
}

#[tauri::command]
pub async fn git_stash_list(path: String) -> Result<Vec<git::Stash>, String> {
    git::stash_list(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn git_stash_push(
    path: String,
    message: String,
    include_untracked: bool,
) -> Result<String, String> {
    git::stash_push(&PathBuf::from(path), &message, include_untracked)
        .map_err(es)
}

#[tauri::command]
pub async fn git_stash_apply(
    path: String,
    reference: String,
    drop: bool,
) -> Result<String, String> {
    git::stash_apply(&PathBuf::from(path), &reference, drop).map_err(es)
}

#[tauri::command]
pub async fn git_stash_drop(
    path: String,
    reference: String,
) -> Result<(), String> {
    git::stash_drop(&PathBuf::from(path), &reference).map_err(es)
}

#[tauri::command]
pub async fn git_tags(path: String) -> Result<Vec<git::Tag>, String> {
    git::tags(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn git_create_tag(
    path: String,
    name: String,
    message: Option<String>,
    target: Option<String>,
) -> Result<(), String> {
    git::create_tag(
        &PathBuf::from(path),
        &name,
        message.as_deref(),
        target.as_deref(),
    )
    .map_err(es)
}

#[tauri::command]
pub async fn git_delete_tag(path: String, name: String) -> Result<(), String> {
    git::delete_tag(&PathBuf::from(path), &name).map_err(es)
}

#[tauri::command]
pub async fn git_push_tag(
    path: String,
    name: String,
) -> Result<String, String> {
    git::push_tag(&PathBuf::from(path), &name).map_err(es)
}

#[tauri::command]
pub async fn git_pack_url(path: String) -> Result<Option<String>, String> {
    Ok(git::pack_remote_url(&PathBuf::from(path)))
}

#[tauri::command]
pub async fn git_remotes(path: String) -> Result<Vec<git::Remote>, String> {
    git::remotes(&PathBuf::from(path)).map_err(es)
}

#[tauri::command]
pub async fn git_add_remote(
    path: String,
    name: String,
    url: String,
) -> Result<(), String> {
    git::add_remote(&PathBuf::from(path), &name, &url).map_err(es)
}

#[tauri::command]
pub async fn git_set_remote_url(
    path: String,
    name: String,
    url: String,
) -> Result<(), String> {
    git::set_remote_url(&PathBuf::from(path), &name, &url).map_err(es)
}

#[tauri::command]
pub async fn git_remove_remote(
    path: String,
    name: String,
) -> Result<(), String> {
    git::remove_remote(&PathBuf::from(path), &name).map_err(es)
}

#[tauri::command]
pub async fn github_release(
    path: String,
    tag: String,
    name: String,
    body: String,
) -> Result<String, String> {
    let dir = PathBuf::from(&path);
    let repo = git::github_repo(&dir).ok_or_else(|| {
        "This pack's origin isn't a GitHub repository.".to_string()
    })?;
    let token = secrets::get("git_token").ok_or_else(|| {
        "Add a GitHub token in Settings (Git access) first.".to_string()
    })?;
    github::create_release(&repo, &token, &tag, &name, &body)
        .await
        .map_err(es)
}
