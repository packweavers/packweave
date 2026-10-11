use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::content::ContentItem;
use crate::git::{self, FileStat, GitChange, GitCommit, GitStatus, GraphCommit, MergeState};
use crate::github;
use crate::gitsetup::{self, GitSetup};
use crate::packdiff::{self, PackDiff};
use crate::secrets;
use crate::undo;
use crate::worktree::{self, Worktree};

use super::{blocking, es, lock_pack};
use ts_rs::TS;

fn dir_of(path: &str) -> PathBuf {
    PathBuf::from(path)
}

async fn rewriting<T, F>(path: String, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> anyhow::Result<T> + Send + 'static,
{
    let _held = lock_pack(&dir_of(&path)).await;
    blocking(move || {
        let dir = dir_of(&path);
        git::reset_index_json(&dir);
        let result = work(&dir);
        undo::forget(&dir);
        result
    })
    .await
}

#[tauri::command]
pub async fn git_status(path: String) -> Result<GitStatus, String> {
    blocking(move || git::status(&dir_of(&path))).await
}

#[tauri::command]
pub async fn git_init(path: String) -> Result<(), String> {
    blocking(move || {
        let dir = dir_of(&path);
        crate::instance::ensure_gitignore(&dir)?;
        git::init(&dir)
    })
    .await
}

#[tauri::command]
pub async fn git_setup_state() -> Result<GitSetup, String> {
    blocking(|| Ok(gitsetup::state())).await
}

#[tauri::command]
pub async fn git_token_save(token: String, host: Option<String>) -> Result<Option<String>, String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("Paste a token first".into());
    }
    let account = github::verify_token(host.as_deref(), &token)
        .await
        .map_err(es)?;
    let key = match host.as_deref() {
        Some(h) if !h.is_empty() => git::token_key(h),
        _ => "git_token".to_string(),
    };
    secrets::set(&key, &token).map_err(es)?;
    Ok(account)
}

#[tauri::command]
pub async fn git_token_delete(host: Option<String>) -> Result<(), String> {
    let key = match host.as_deref() {
        Some(h) if !h.is_empty() => git::token_key(h),
        _ => "git_token".to_string(),
    };
    secrets::delete(&key).map_err(es)
}

#[tauri::command]
pub async fn git_setup_recheck() -> Result<GitSetup, String> {
    blocking(|| Ok(gitsetup::recheck())).await
}

#[tauri::command]
pub async fn git_setup_install() -> Result<GitSetup, String> {
    blocking(gitsetup::install).await
}

#[tauri::command]
pub async fn git_set_identity(name: String, email: String) -> Result<GitSetup, String> {
    blocking(move || {
        git::set_identity(&name, &email)?;
        Ok(gitsetup::state())
    })
    .await
}

#[tauri::command]
pub async fn git_commit(
    path: String,
    message: String,
    files: Vec<String>,
    amend: bool,
) -> Result<(), String> {
    let _held = lock_pack(&dir_of(&path)).await;
    blocking(move || {
        let dir = dir_of(&path);
        git::commit(&dir, &message, &files, amend)?;
        crate::content::write_index(&dir)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn git_log(
    path: String,
    limit: u32,
    refname: Option<String>,
    file: Option<String>,
) -> Result<Vec<GitCommit>, String> {
    blocking(move || git::log(&dir_of(&path), limit, refname.as_deref(), file.as_deref())).await
}

#[tauri::command]
pub async fn git_graph(
    path: String,
    limit: u32,
    skip: u32,
    all: bool,
) -> Result<Vec<GraphCommit>, String> {
    blocking(move || git::graph(&dir_of(&path), limit, skip, all)).await
}

#[tauri::command]
pub async fn git_push(path: String, force: bool, tags: bool) -> Result<String, String> {
    blocking(move || git::push(&dir_of(&path), force, tags)).await
}

#[tauri::command]
pub async fn git_pull(path: String, strategy: String) -> Result<String, String> {
    rewriting(path, move |dir| git::pull(dir, &strategy)).await
}

#[tauri::command]
pub async fn git_fetch(path: String) -> Result<String, String> {
    blocking(move || git::fetch(&dir_of(&path))).await
}

#[tauri::command]
pub async fn git_discard_files(path: String, files: Vec<String>) -> Result<(), String> {
    rewriting(path, move |dir| git::revert_files(dir, &files)).await
}

#[tauri::command]
pub async fn git_diff_file(path: String, file: String, staged: bool) -> Result<String, String> {
    blocking(move || git::diff_file(&dir_of(&path), &file, staged)).await
}

#[tauri::command]
pub async fn git_working_stats(path: String) -> Result<Vec<FileStat>, String> {
    blocking(move || {
        Ok(git::working_stats(&dir_of(&path))
            .into_iter()
            .map(|(path, (added, removed))| FileStat {
                path,
                status: String::new(),
                added,
                removed,
            })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn git_branches(path: String) -> Result<git::Branches, String> {
    blocking(move || git::branches(&dir_of(&path))).await
}

#[tauri::command]
pub async fn git_checkout(path: String, branch: String) -> Result<(), String> {
    rewriting(path, move |dir| git::checkout(dir, &branch)).await
}

#[tauri::command]
pub async fn git_create_branch(
    path: String,
    name: String,
    start_point: Option<String>,
    checkout: bool,
) -> Result<(), String> {
    let _held = lock_pack(&dir_of(&path)).await;
    blocking(move || git::create_branch(&dir_of(&path), &name, start_point.as_deref(), checkout))
        .await
}

#[tauri::command]
pub async fn git_rename_branch(path: String, old: String, new: String) -> Result<(), String> {
    blocking(move || git::rename_branch(&dir_of(&path), &old, &new)).await
}

#[tauri::command]
pub async fn git_delete_branch(path: String, name: String, force: bool) -> Result<(), String> {
    blocking(move || {
        let dir = dir_of(&path);
        if let Some(tree) = worktree::list(&dir)?
            .into_iter()
            .find(|w| !w.main && w.branch.as_deref() == Some(name.as_str()))
        {
            worktree::remove(&dir, Path::new(&tree.path), force)?;
        }
        git::delete_branch(&dir, &name, force)
    })
    .await
}

#[tauri::command]
pub async fn git_delete_remote_branch(path: String, name: String) -> Result<String, String> {
    blocking(move || git::delete_remote_branch(&dir_of(&path), &name)).await
}

#[tauri::command]
pub async fn git_rebase(path: String, name: String) -> Result<String, String> {
    rewriting(path, move |dir| git::rebase_branch(dir, &name)).await
}

#[tauri::command]
pub async fn git_push_branch(
    path: String,
    remote: String,
    branch: String,
    set_upstream: bool,
) -> Result<String, String> {
    blocking(move || git::push_branch(&dir_of(&path), &remote, &branch, set_upstream)).await
}

#[tauri::command]
pub async fn git_merge(
    path: String,
    from: String,
    mode: String,
    message: String,
) -> Result<bool, String> {
    rewriting(path, move |dir| git::merge(dir, &from, &mode, &message)).await
}

#[tauri::command]
pub async fn git_merge_state(path: String) -> Result<MergeState, String> {
    blocking(move || Ok(git::merge_state(&dir_of(&path)))).await
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct ConflictSides {
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub ours_item: Option<ContentItem>,
    pub theirs_item: Option<ContentItem>,
}

#[tauri::command]
pub async fn git_conflict_sides(path: String, file: String) -> Result<ConflictSides, String> {
    blocking(move || {
        let dir = dir_of(&path);
        let ours = git::conflict_side(&dir, &file, 2);
        let theirs = git::conflict_side(&dir, &file, 3);
        let parse = |t: &Option<String>| -> Option<ContentItem> {
            if !file.starts_with("content/") {
                return None;
            }
            let mut item: ContentItem = serde_json::from_str(t.as_deref()?).ok()?;
            item.project_type = crate::ptype::ProjectType::from_content_path(&file);
            Some(item)
        };
        Ok(ConflictSides {
            ours_item: parse(&ours),
            theirs_item: parse(&theirs),
            ours,
            theirs,
        })
    })
    .await
}

#[tauri::command]
pub async fn git_resolve_conflict(path: String, file: String, side: String) -> Result<(), String> {
    let _held = lock_pack(&dir_of(&path)).await;
    blocking(move || git::resolve_conflict(&dir_of(&path), &file, &side)).await
}

#[tauri::command]
pub async fn git_resolve_with(path: String, file: String, text: String) -> Result<(), String> {
    let _held = lock_pack(&dir_of(&path)).await;
    blocking(move || git::resolve_with(&dir_of(&path), &file, &text)).await
}

#[tauri::command]
pub async fn git_resolve_content_version(
    path: String,
    file: String,
    version: String,
) -> Result<(), String> {
    let _held = lock_pack(&dir_of(&path)).await;
    blocking(move || {
        let dir = dir_of(&path);
        let text = git::conflict_side(&dir, &file, 2)
            .or_else(|| git::conflict_side(&dir, &file, 3))
            .ok_or_else(|| anyhow::anyhow!("{file} has no version to start from"))?;
        let mut item: ContentItem = serde_json::from_str(&text)?;
        let preferred = item.preferred;
        if let Some(src) = item.sources.get_mut(&preferred) {
            src.version_id = version;
            src.version_number.clear();
            src.filename.clear();
            src.download_url.clear();
            src.sha1 = None;
            src.sha512 = None;
            src.file_size = 0;
        }
        git::resolve_with(
            &dir,
            &file,
            &format!("{}\n", serde_json::to_string_pretty(&item)?),
        )
    })
    .await
}

#[tauri::command]
pub async fn git_merge_abort(path: String) -> Result<(), String> {
    rewriting(path, git::merge_abort).await
}

#[tauri::command]
pub async fn git_merge_commit(path: String, message: String) -> Result<(), String> {
    blocking(move || git::merge_commit(&dir_of(&path), &message)).await
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Compare {
    pub base: String,
    pub other: String,
    pub commits: Vec<GraphCommit>,
    pub behind: u32,
    pub files: Vec<FileStat>,
    pub pack: Option<PackDiff>,
    pub conflicts: Option<Vec<String>>,
}

#[tauri::command]
pub async fn git_compare(path: String, base: String, other: String) -> Result<Compare, String> {
    blocking(move || {
        let dir = dir_of(&path);
        let fork = git::merge_base(&dir, &base, &other).unwrap_or_else(|| base.clone());
        Ok(Compare {
            commits: git::commits_between(&dir, &base, &other)?,
            behind: git::rev_count(&dir, &format!("{other}..{base}")),
            files: git::diff_stats(&dir, &base, &other)?,
            pack: packdiff::pack_diff(&dir, &fork, &other).ok(),
            conflicts: git::merge_preview(&dir, &base, &other),
            base,
            other,
        })
    })
    .await
}

#[tauri::command]
pub async fn git_commit_changes(path: String, hash: String) -> Result<Vec<GitChange>, String> {
    blocking(move || git::commit_changes(&dir_of(&path), &hash)).await
}

#[tauri::command]
pub async fn git_show_diff(path: String, hash: String, file: String) -> Result<String, String> {
    blocking(move || git::show_diff(&dir_of(&path), &hash, &file)).await
}

#[tauri::command]
pub async fn git_show_file(
    path: String,
    reference: String,
    file: String,
) -> Result<String, String> {
    blocking(move || git::show_file(&dir_of(&path), &reference, &file)).await
}

#[tauri::command]
pub async fn git_pack_diff(path: String, from: String, to: String) -> Result<PackDiff, String> {
    blocking(move || packdiff::pack_diff(&dir_of(&path), &from, &to)).await
}

#[tauri::command]
pub async fn git_pack_diff_working(path: String) -> Result<PackDiff, String> {
    blocking(move || packdiff::pack_diff_working(&dir_of(&path))).await
}

#[tauri::command]
pub async fn git_latest_tag(path: String) -> Result<Option<String>, String> {
    blocking(move || Ok(git::latest_tag(&dir_of(&path)))).await
}

const CHANGELOG_HEADER: &str = "# Changelog\n\n";

#[tauri::command]
pub async fn changelog_save(path: String, section: String) -> Result<(), String> {
    let file = dir_of(&path).join("CHANGELOG.md");
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
pub async fn git_revert_commit(path: String, hash: String) -> Result<String, String> {
    rewriting(path, move |dir| git::revert_commit(dir, &hash)).await
}

#[tauri::command]
pub async fn git_reset(path: String, hash: String, mode: String) -> Result<(), String> {
    rewriting(path, move |dir| git::reset(dir, &hash, &mode)).await
}

#[tauri::command]
pub async fn git_cherry_pick(path: String, hash: String) -> Result<String, String> {
    rewriting(path, move |dir| git::cherry_pick(dir, &hash)).await
}

#[tauri::command]
pub async fn git_stash_list(path: String) -> Result<Vec<git::Stash>, String> {
    blocking(move || git::stash_list(&dir_of(&path))).await
}

#[tauri::command]
pub async fn git_stash_push(
    path: String,
    message: String,
    include_untracked: bool,
) -> Result<String, String> {
    rewriting(path, move |dir| {
        git::stash_push(dir, &message, include_untracked)
    })
    .await
}

#[tauri::command]
pub async fn git_stash_apply(
    path: String,
    reference: String,
    drop: bool,
) -> Result<String, String> {
    rewriting(path, move |dir| git::stash_apply(dir, &reference, drop)).await
}

#[tauri::command]
pub async fn git_stash_drop(path: String, reference: String) -> Result<(), String> {
    blocking(move || git::stash_drop(&dir_of(&path), &reference)).await
}

#[tauri::command]
pub async fn git_tags(path: String) -> Result<Vec<git::Tag>, String> {
    blocking(move || git::tags(&dir_of(&path))).await
}

#[tauri::command]
pub async fn git_upstream_head(path: String) -> Result<Option<String>, String> {
    blocking(move || Ok(git::upstream_head(&dir_of(&path)))).await
}

#[tauri::command]
pub async fn git_mark_tag(path: String, name: String, commit: String) -> Result<(), String> {
    blocking(move || git::mark_tag(&dir_of(&path), &name, &commit)).await
}

#[tauri::command]
pub async fn git_create_tag(
    path: String,
    name: String,
    message: Option<String>,
    target: Option<String>,
) -> Result<(), String> {
    blocking(move || git::create_tag(&dir_of(&path), &name, message.as_deref(), target.as_deref()))
        .await
}

#[tauri::command]
pub async fn git_delete_tag(path: String, name: String) -> Result<(), String> {
    blocking(move || git::delete_tag(&dir_of(&path), &name)).await
}

#[tauri::command]
pub async fn git_push_tag(path: String, name: String) -> Result<String, String> {
    blocking(move || git::push_tag(&dir_of(&path), &name)).await
}

#[tauri::command]
pub async fn git_pack_url(path: String) -> Result<Option<String>, String> {
    blocking(move || Ok(git::pack_remote_url(&dir_of(&path)))).await
}

#[tauri::command]
pub async fn git_github_repo(path: String) -> Result<Option<String>, String> {
    blocking(move || Ok(git::github_repo(&dir_of(&path)))).await
}

#[tauri::command]
pub async fn git_remotes(path: String) -> Result<Vec<git::Remote>, String> {
    blocking(move || git::remotes(&dir_of(&path))).await
}

#[tauri::command]
pub async fn git_add_remote(path: String, name: String, url: String) -> Result<(), String> {
    blocking(move || git::add_remote(&dir_of(&path), &name, &url)).await
}

#[tauri::command]
pub async fn git_set_remote_url(path: String, name: String, url: String) -> Result<(), String> {
    blocking(move || git::set_remote_url(&dir_of(&path), &name, &url)).await
}

#[tauri::command]
pub async fn git_remove_remote(path: String, name: String) -> Result<(), String> {
    blocking(move || git::remove_remote(&dir_of(&path), &name)).await
}

#[tauri::command]
pub async fn worktree_list(path: String) -> Result<Vec<Worktree>, String> {
    blocking(move || worktree::list(&dir_of(&path))).await
}

#[tauri::command]
pub async fn worktree_main(path: String) -> Result<String, String> {
    blocking(move || {
        Ok(worktree::main_dir(&dir_of(&path))
            .to_string_lossy()
            .to_string())
    })
    .await
}

#[tauri::command]
pub async fn branch_folder(
    path: String,
    branch: String,
    start: Option<String>,
) -> Result<String, String> {
    blocking(move || {
        let folder = worktree::open_branch(&dir_of(&path), &branch, start.as_deref())?;
        crate::instance::ensure_defaults(&folder)?;
        Ok(folder.to_string_lossy().to_string())
    })
    .await
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct GithubAccess {
    pub origin: Option<String>,
    pub repo: Option<String>,
    pub signed_in: bool,
    pub visibility: github::Visibility,
}

#[tauri::command]
pub async fn github_access(path: String, repo: Option<String>) -> Result<GithubAccess, String> {
    let dir = dir_of(&path);
    let (origin, token) = super::github_identity(&dir).await;
    let repo = repo
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .or_else(|| origin.clone());
    let visibility = match &repo {
        Some(repo) => github::repo_visibility(repo, token.as_deref()).await,
        None => github::Visibility::Unknown,
    };
    Ok(GithubAccess {
        origin,
        repo,
        signed_in: token.is_some(),
        visibility,
    })
}

#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct GithubRelease {
    pub repo: String,
    pub tag: String,
    pub name: String,
    pub body: String,
    pub updates: bool,
    pub url: Option<String>,
    pub prerelease: bool,
    pub assets: Vec<String>,
    pub target: Option<String>,
}

#[tauri::command]
pub async fn github_release(path: String, release: GithubRelease) -> Result<String, String> {
    let dir = dir_of(&path);
    if !github::is_repo_slug(&release.repo) {
        return Err(format!(
            "{} isn't a GitHub repository. Use owner/name.",
            release.repo
        ));
    }
    let (_, token) = super::github_identity(&dir).await;
    let token = token.ok_or_else(|| {
        "Sign in to GitHub first: use Git once, or add a token in Settings › Accounts.".to_string()
    })?;
    let mut bundles: Vec<(String, Vec<u8>)> = Vec::new();
    if release.updates {
        let dist = super::saved_dist_token(&dir);
        let url = release
            .url
            .clone()
            .filter(|u| !u.trim().is_empty())
            .unwrap_or_else(|| format!("release+https://github.com/{}", release.repo));
        let snapshot = crate::release::build_snapshot(&dir, &url, dist.as_deref()).map_err(es)?;
        bundles.push(("pack.zip".into(), snapshot));
        if let Some(prev) = git::previous_tag(&dir, &release.tag) {
            if let Some(delta) = crate::release::build_delta(&dir, &prev, &url, dist.as_deref()) {
                bundles.push((format!("delta-{prev}.zip"), delta));
            }
        }
    }
    let rel = github::create_release(
        &release.repo,
        &token,
        &release.tag,
        &release.name,
        &release.body,
        release.prerelease,
        release.target.as_deref(),
    )
    .await
    .map_err(es)?;
    for (name, bytes) in bundles {
        github::upload_asset(&release.repo, &token, rel.id, &name, bytes)
            .await
            .map_err(es)?;
    }
    for asset in &release.assets {
        let file = PathBuf::from(asset);
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| format!("{asset} isn't a file"))?;
        let bytes = blocking(move || Ok(std::fs::read(&file)?)).await?;
        github::upload_asset(&release.repo, &token, rel.id, &name, bytes)
            .await
            .map_err(es)?;
    }
    Ok(rel.html_url)
}
