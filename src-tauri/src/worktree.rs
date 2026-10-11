use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::git;
use crate::packlocal;
use ts_rs::TS;

#[derive(Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Worktree {
    pub path: String,
    pub branch: Option<String>,
    pub head: String,
    pub main: bool,
    pub instance: Option<String>,
}

static BRANCHES_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_dir(dir: PathBuf) {
    let _ = BRANCHES_DIR.set(dir);
}

fn branches_dir() -> Result<&'static PathBuf> {
    BRANCHES_DIR
        .get()
        .ok_or_else(|| anyhow!("the branch folder isn't set up"))
}

pub fn list(dir: &Path) -> Result<Vec<Worktree>> {
    if !git::is_repo(dir) {
        return Ok(Vec::new());
    }
    let out = git::run(dir, &["worktree", "list", "--porcelain"])?;
    let mut list: Vec<Worktree> = Vec::new();
    for block in out.split("\n\n") {
        let mut path = None;
        let mut head = String::new();
        let mut branch = None;
        let mut bare = false;
        for line in block.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                path = Some(p.to_string());
            } else if let Some(h) = line.strip_prefix("HEAD ") {
                head = h.to_string();
            } else if let Some(b) = line.strip_prefix("branch ") {
                branch = Some(b.trim_start_matches("refs/heads/").to_string());
            } else if line == "bare" {
                bare = true;
            }
        }
        let Some(path) = path else {
            continue;
        };
        if bare || in_builds(Path::new(&path)) {
            continue;
        }
        let main = list.is_empty();
        let instance = packlocal::read(Path::new(&path)).instance_dir;
        list.push(Worktree {
            path,
            branch,
            head,
            main,
            instance,
        });
    }
    Ok(list)
}

fn builds_dir() -> PathBuf {
    std::env::temp_dir().join("packweave-build")
}

fn in_builds(path: &Path) -> bool {
    let root = builds_dir();
    (path != root && path.starts_with(&root)) || is_build(path)
}

fn is_build(path: &Path) -> bool {
    match (
        std::fs::canonicalize(builds_dir()),
        std::fs::canonicalize(path),
    ) {
        (Ok(root), Ok(path)) => path != root && path.starts_with(&root),
        _ => false,
    }
}

pub fn checkout_commit(pack_dir: &Path, commit: &str) -> Result<PathBuf> {
    if commit.len() < 7 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("{commit} isn't a commit");
    }
    let main = main_dir(pack_dir);
    let root = builds_dir();
    for entry in std::fs::read_dir(&root).into_iter().flatten().flatten() {
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age.as_secs() > 3600);
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    let _ = git::run(&main, &["worktree", "prune"]);
    std::fs::create_dir_all(&root)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let folder = root.join(stamp.to_string());
    git::run(
        &main,
        &[
            "worktree",
            "add",
            "--detach",
            &folder.to_string_lossy(),
            commit,
        ],
    )?;
    crate::content::write_index(&folder)?;
    Ok(folder)
}

pub fn remove_build(pack_dir: &Path, folder: &Path) -> Result<()> {
    if !is_build(folder) {
        bail!("{} isn't a release build folder", folder.display());
    }
    let main = main_dir(pack_dir);
    let path = folder.to_string_lossy().to_string();
    if git::run(&main, &["worktree", "remove", "--force", &path]).is_err() {
        std::fs::remove_dir_all(folder)?;
        let _ = git::run(&main, &["worktree", "prune"]);
    }
    Ok(())
}

pub fn main_dir(dir: &Path) -> PathBuf {
    list(dir)
        .ok()
        .and_then(|l| l.into_iter().find(|w| w.main))
        .map(|w| PathBuf::from(w.path))
        .unwrap_or_else(|| dir.to_path_buf())
}

fn folder_for(main: &Path, branch: &str) -> Result<PathBuf> {
    let pack = main
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "pack".into());
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '-'
                }
            })
            .collect()
    };
    let base = format!("{}-{}", clean(&pack), clean(branch));
    let root = branches_dir()?;
    std::fs::create_dir_all(root)?;
    let mut candidate = root.join(&base);
    let mut n = 2;
    while candidate.exists() {
        candidate = root.join(format!("{base}-{n}"));
        n += 1;
    }
    Ok(candidate)
}

pub fn open_branch(dir: &Path, branch: &str, start: Option<&str>) -> Result<PathBuf> {
    let branch = branch.trim();
    if branch.is_empty() {
        bail!("Enter a branch name");
    }
    let main = main_dir(dir);
    let (local, remote) = match git::split_remote_branch(&main, branch) {
        Some(local) => (local, Some(branch.to_string())),
        None => (branch.to_string(), git::remote_tracking_for(&main, branch)),
    };
    if let Some(existing) = list(&main)?
        .into_iter()
        .find(|w| w.branch.as_deref() == Some(local.as_str()))
    {
        return Ok(PathBuf::from(existing.path));
    }
    let folder = folder_for(&main, &local)?;
    let folder_str = folder.to_string_lossy().to_string();
    if git::branch_exists(&main, &local) {
        git::run(&main, &["worktree", "add", &folder_str, &local])?;
    } else if let Some(remote) = remote {
        git::run(
            &main,
            &[
                "worktree",
                "add",
                "--track",
                "-b",
                &local,
                &folder_str,
                &remote,
            ],
        )?;
    } else {
        let mut args = vec!["worktree", "add", "-b", local.as_str(), folder_str.as_str()];
        if let Some(s) = start.filter(|s| !s.trim().is_empty()) {
            args.push(s);
        }
        git::run(&main, &args)?;
    }
    Ok(folder)
}

pub fn remove(dir: &Path, folder: &Path, force: bool) -> Result<()> {
    let main = main_dir(dir);
    if main == folder {
        bail!("The main folder can't be removed.");
    }
    let folder_str = folder.to_string_lossy().to_string();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&folder_str);
    git::run(&main, &args)?;
    let _ = git::run(&main, &["worktree", "prune"]);
    Ok(())
}
