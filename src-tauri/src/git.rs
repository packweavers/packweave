use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::path::Path;
use std::process::Command;

use crate::secrets;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub detached: bool,
    pub ahead: u32,
    pub behind: u32,
    pub has_remote: bool,
    pub clean: bool,
    pub conflicts: u32,
    pub changes: Vec<GitChange>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitChange {
    pub status: String,
    pub path: String,
    pub staged: bool,
    pub conflicted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommit {
    pub hash: String,
    pub short: String,
    pub subject: String,
    pub author: String,
    pub email: String,
    pub relative: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub name: String,
    pub current: bool,
    pub remote: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub gone: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Branches {
    pub current: Option<String>,
    pub detached: bool,
    pub list: Vec<Branch>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    pub name: String,
    pub url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stash {
    pub reference: String,
    pub message: String,
    pub branch: String,
    pub relative: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub name: String,
    pub subject: String,
}

fn new_git() -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new("git");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = new_git()
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| anyhow!("git is not installed or not on PATH: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!("git {}: {}", args.join(" "), err.trim()))
    }
}

fn git_ok(dir: &Path, args: &[&str]) -> bool {
    new_git()
        .current_dir(dir)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn net_command(dir: Option<&Path>) -> Command {
    let mut cmd = new_git();
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    if let Some(token) =
        secrets::get("git_token").filter(|t| !t.trim().is_empty())
    {
        if let Ok(exe) = std::env::current_exe() {
            cmd.arg("-c").arg("credential.helper=");
            cmd.env("GIT_ASKPASS", exe)
                .env("PACKWEAVE_ASKPASS", "1")
                .env("PACKWEAVE_GIT_USER", "x-access-token")
                .env("PACKWEAVE_GIT_TOKEN", token.trim())
                .env("GCM_INTERACTIVE", "never");
        }
    }
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

fn git_net(dir: &Path, args: &[&str]) -> Result<String> {
    let mut cmd = net_command(Some(dir));
    cmd.args(args);
    let output = cmd
        .output()
        .map_err(|e| anyhow!("git is not installed or not on PATH: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(net_error(
            args,
            String::from_utf8_lossy(&output.stderr).trim(),
        ))
    }
}

fn net_error(args: &[&str], stderr: &str) -> anyhow::Error {
    let low = stderr.to_lowercase();
    let auth = low.contains("authentication failed")
        || low.contains("could not read username")
        || low.contains("terminal prompts disabled")
        || low.contains("invalid username or password")
        || low.contains("permission denied")
        || low.contains("not found")
        || low.contains("error: 401")
        || low.contains("error: 403")
        || low.contains("error: 404");
    if auth {
        return anyhow!(
            "GIT_AUTH:Authentication required for this repository."
        );
    }
    anyhow!("git {}: {}", args.join(" "), stderr)
}

pub fn clone(url: &str, dest: &Path) -> Result<()> {
    let mut cmd = net_command(None);
    cmd.arg("clone").arg(url).arg(dest);
    let output = cmd
        .output()
        .map_err(|e| anyhow!("git is not installed or not on PATH: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(net_error(
            &["clone"],
            String::from_utf8_lossy(&output.stderr).trim(),
        ))
    }
}

pub fn is_repo(dir: &Path) -> bool {
    git(dir, &["rev-parse", "--is-inside-work-tree"])
        .map(|s| s.trim() == "true")
        .unwrap_or(false)
}

pub fn init(dir: &Path) -> Result<()> {
    git(dir, &["init"])?;
    Ok(())
}

fn remote_names(dir: &Path) -> Vec<String> {
    git(dir, &["remote"])
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn ref_exists(dir: &Path, full_ref: &str) -> bool {
    git_ok(dir, &["show-ref", "--verify", "--quiet", full_ref])
}

fn has_head(dir: &Path) -> bool {
    git_ok(dir, &["rev-parse", "--verify", "-q", "HEAD"])
}

fn current_branch(dir: &Path) -> Option<String> {
    git(dir, &["symbolic-ref", "--short", "-q", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn mid_operation(dir: &Path) -> bool {
    [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "REBASE_HEAD",
    ]
    .iter()
    .any(|r| git_ok(dir, &["rev-parse", "--verify", "-q", r]))
}

pub fn status(dir: &Path) -> Result<GitStatus> {
    if !is_repo(dir) {
        return Ok(GitStatus {
            is_repo: false,
            branch: None,
            upstream: None,
            detached: false,
            ahead: 0,
            behind: 0,
            has_remote: false,
            clean: true,
            conflicts: 0,
            changes: vec![],
        });
    }

    let porcelain = git(
        dir,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--branch",
            "--untracked-files=all",
        ],
    )?;
    let mut branch = None;
    let mut upstream = None;
    let mut ahead = 0;
    let mut behind = 0;
    let mut changes = Vec::new();
    let mut conflicts = 0;

    let records: Vec<&str> = porcelain.split('\0').collect();
    let mut i = 0;
    while i < records.len() {
        let rec = records[i].trim_end_matches(['\n', '\r']);
        i += 1;
        if rec.is_empty() {
            continue;
        }
        if let Some(rest) = rec.strip_prefix("## ") {
            let head = rest.split('[').next().unwrap_or(rest).trim();
            let head = head.strip_prefix("No commits yet on ").unwrap_or(head);
            let (local, up) = match head.split_once("...") {
                Some((l, u)) => (l, Some(u.trim().to_string())),
                None => (head, None),
            };
            if !local.starts_with("HEAD") && !local.is_empty() {
                branch = Some(local.trim().to_string());
            }
            upstream = up;
            if let Some(start) = rest.find('[') {
                let bracket = &rest[start..];
                ahead = parse_count(bracket, "ahead");
                behind = parse_count(bracket, "behind");
            }
            continue;
        }
        if rec.len() < 3 {
            continue;
        }
        let x = &rec[0..1];
        let y = &rec[1..2];
        let path = rec[3..].to_string();
        if x == "R" || y == "R" || x == "C" || y == "C" {
            i += 1;
        }
        if path.is_empty() {
            continue;
        }
        let conflicted = x == "U"
            || y == "U"
            || (x == "A" && y == "A")
            || (x == "D" && y == "D");
        if conflicted {
            conflicts += 1;
        }
        let staged = x != " " && x != "?" && !conflicted;
        let status = if conflicted {
            "U".to_string()
        } else if x == "?" {
            "?".to_string()
        } else if x != " " {
            x.to_string()
        } else {
            y.to_string()
        };
        changes.push(GitChange {
            status,
            path,
            staged,
            conflicted,
        });
    }

    if branch.is_none() {
        branch = current_branch(dir);
    }
    let detached = branch.is_none() && has_head(dir);
    let has_remote = !remote_names(dir).is_empty();
    let clean = changes.is_empty();

    Ok(GitStatus {
        is_repo: true,
        branch,
        upstream,
        detached,
        ahead,
        behind,
        has_remote,
        clean,
        conflicts,
        changes,
    })
}

fn parse_count(text: &str, key: &str) -> u32 {
    text.find(key)
        .map(|idx| {
            text[idx + key.len()..]
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

pub fn branches(dir: &Path) -> Result<Branches> {
    if !is_repo(dir) {
        return Ok(Branches {
            current: None,
            detached: false,
            list: vec![],
        });
    }
    let current = current_branch(dir);
    let detached = current.is_none() && has_head(dir);

    let fmt = "%(HEAD)%00%(refname)%00%(refname:short)%00%(upstream:short)%00%(upstream:track)";
    let out = git(
        dir,
        &[
            "for-each-ref",
            &format!("--format={fmt}"),
            "refs/heads",
            "refs/remotes",
        ],
    )
    .unwrap_or_default();

    let mut list: Vec<Branch> = Vec::new();
    for line in out.lines() {
        let line = line.trim_matches(['\n', '\r']);
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\0').collect();
        if parts.len() < 3 {
            continue;
        }
        let full_ref = parts[1];
        let short = parts[2].to_string();
        let remote = full_ref.starts_with("refs/remotes/");
        if remote && short.ends_with("/HEAD") {
            continue;
        }
        let upstream = parts
            .get(3)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let track = parts.get(4).copied().unwrap_or("");
        list.push(Branch {
            name: short.clone(),
            current: !remote && current.as_deref() == Some(short.as_str()),
            remote,
            upstream,
            ahead: parse_count(track, "ahead"),
            behind: parse_count(track, "behind"),
            gone: track.contains("gone"),
        });
    }

    if let Some(cur) = &current {
        if !list.iter().any(|b| !b.remote && &b.name == cur) {
            list.insert(
                0,
                Branch {
                    name: cur.clone(),
                    current: true,
                    remote: false,
                    upstream: None,
                    ahead: 0,
                    behind: 0,
                    gone: false,
                },
            );
        }
    }
    list.sort_by(|a, b| {
        a.remote.cmp(&b.remote).then_with(|| a.name.cmp(&b.name))
    });

    Ok(Branches {
        current,
        detached,
        list,
    })
}

fn local_name_of_remote(dir: &Path, name: &str) -> Option<String> {
    for r in remote_names(dir) {
        if let Some(rest) = name.strip_prefix(&format!("{r}/")) {
            if rest != "HEAD" && !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

pub fn checkout(dir: &Path, name: &str) -> Result<()> {
    if ref_exists(dir, &format!("refs/heads/{name}")) {
        git(dir, &["checkout", name])?;
    } else if let Some(local) = local_name_of_remote(dir, name) {
        if ref_exists(dir, &format!("refs/heads/{local}")) {
            git(dir, &["checkout", &local])?;
        } else {
            git(dir, &["checkout", "-b", &local, "--track", name])?;
        }
    } else {
        git(dir, &["checkout", name])?;
    }
    Ok(())
}

pub fn create_branch(
    dir: &Path,
    name: &str,
    start_point: Option<&str>,
    checkout_it: bool,
) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        bail!("Enter a branch name");
    }
    let mut args = vec![if checkout_it { "checkout" } else { "branch" }];
    if checkout_it {
        args.push("-b");
    }
    args.push(name);
    if let Some(sp) = start_point.filter(|s| !s.trim().is_empty()) {
        args.push(sp);
    }
    git(dir, &args)?;
    Ok(())
}

pub fn rename_branch(dir: &Path, old: &str, new: &str) -> Result<()> {
    let new = new.trim();
    if new.is_empty() {
        bail!("Enter a branch name");
    }
    git(dir, &["branch", "-m", old, new])?;
    Ok(())
}

pub fn delete_branch(dir: &Path, name: &str, force: bool) -> Result<()> {
    git(dir, &["branch", if force { "-D" } else { "-d" }, name])?;
    Ok(())
}

pub fn delete_remote_branch(dir: &Path, name: &str) -> Result<String> {
    let (remote, branch) = name
        .split_once('/')
        .ok_or_else(|| anyhow!("Not a remote branch: {name}"))?;
    git_net(dir, &["push", remote, "--delete", branch])
}

pub fn merge_branch(dir: &Path, name: &str) -> Result<String> {
    git(dir, &["merge", "--no-edit", name])
}

pub fn rebase_branch(dir: &Path, name: &str) -> Result<String> {
    git(dir, &["rebase", name])
}

pub fn set_upstream(dir: &Path, upstream: &str) -> Result<()> {
    git(dir, &["branch", "--set-upstream-to", upstream])?;
    Ok(())
}

pub fn commit(
    dir: &Path,
    message: &str,
    files: &[String],
    amend: bool,
) -> Result<()> {
    if message.trim().is_empty() {
        bail!("Enter a commit message");
    }
    if !amend && files.is_empty() {
        bail!("Select at least one change to commit");
    }
    if mid_operation(dir) {
        let args: &[&str] = if amend {
            &["commit", "--amend", "-m", message]
        } else {
            &["commit", "-m", message]
        };
        git(dir, args)?;
        return Ok(());
    }
    if has_head(dir) {
        let _ = git(dir, &["reset", "-q"]);
    } else {
        let _ = git(dir, &["rm", "-r", "--cached", "-q", "--", "."]);
    }
    if !files.is_empty() {
        let mut add = vec!["add", "-A", "--"];
        for f in files {
            add.push(f.as_str());
        }
        git(dir, &add)?;
    }
    if amend {
        git(dir, &["commit", "--amend", "-m", message])?;
    } else {
        git(dir, &["commit", "-m", message])?;
    }
    Ok(())
}

pub fn fetch(dir: &Path) -> Result<String> {
    let remotes = remote_names(dir);
    if remotes.is_empty() {
        bail!("No remote to fetch from.");
    }
    let ordered: Vec<&String> =
        std::iter::once(remotes.iter().find(|r| *r == "origin"))
            .flatten()
            .chain(remotes.iter().filter(|r| *r != "origin"))
            .collect();
    let mut out = String::new();
    for r in ordered {
        out.push_str(&git_net(dir, &["fetch", "--prune", "--tags", r])?);
    }
    Ok(out)
}

pub fn pull(dir: &Path, strategy: &str) -> Result<String> {
    let flag = match strategy {
        "rebase" => "--rebase",
        "merge" => "--no-rebase",
        _ => "--ff-only",
    };
    git_net(dir, &["pull", flag])
}

pub fn push(dir: &Path, force: bool, tags: bool) -> Result<String> {
    let no_upstream = !git_ok(dir, &["rev-parse", "--abbrev-ref", "@{u}"]);
    if no_upstream {
        let branch = current_branch(dir);
        let remotes = remote_names(dir);
        let remote = remotes
            .iter()
            .find(|r| *r == "origin")
            .or_else(|| remotes.first());
        if let (Some(b), Some(r)) = (branch, remote) {
            let mut args = vec!["push"];
            if force {
                args.push("--force-with-lease");
            }
            if tags {
                args.push("--follow-tags");
            }
            args.push("-u");
            args.push(r);
            args.push(&b);
            return git_net(dir, &args);
        }
    }
    let mut args = vec!["push"];
    if force {
        args.push("--force-with-lease");
    }
    if tags {
        args.push("--follow-tags");
    }
    git_net(dir, &args)
}

pub fn push_branch(
    dir: &Path,
    remote: &str,
    branch: &str,
    set_upstream: bool,
) -> Result<String> {
    let mut args = vec!["push"];
    if set_upstream {
        args.push("-u");
    }
    args.push(remote);
    args.push(branch);
    git_net(dir, &args)
}

pub fn log(
    dir: &Path,
    limit: u32,
    refname: Option<&str>,
    path: Option<&str>,
) -> Result<Vec<GitCommit>> {
    if !is_repo(dir) || !has_head(dir) {
        return Ok(vec![]);
    }
    let format = "%H%x1f%h%x1f%s%x1f%an%x1f%ae%x1f%cr%x1e";
    let count = format!("-{limit}");
    let pretty = format!("--pretty=format:{format}");
    let mut args = vec!["log", count.as_str(), pretty.as_str()];
    if let Some(r) = refname.filter(|r| !r.trim().is_empty()) {
        args.push(r);
    }
    if let Some(p) = path.filter(|p| !p.trim().is_empty()) {
        args.push("--");
        args.push(p);
    }
    let out = git(dir, &args).unwrap_or_default();

    let mut commits = Vec::new();
    for record in out.split('\u{1e}') {
        let record = record.trim_matches(['\n', '\r']);
        if record.is_empty() {
            continue;
        }
        let parts: Vec<&str> = record.split('\u{1f}').collect();
        if parts.len() >= 6 {
            commits.push(GitCommit {
                hash: parts[0].into(),
                short: parts[1].into(),
                subject: parts[2].into(),
                author: parts[3].into(),
                email: parts[4].into(),
                relative: parts[5].into(),
            });
        }
    }
    Ok(commits)
}

pub fn commit_changes(dir: &Path, hash: &str) -> Result<Vec<GitChange>> {
    let out = git(dir, &["show", "--name-status", "-z", "--format=", hash])
        .unwrap_or_default();
    let records: Vec<&str> = out.split('\0').collect();
    let mut changes = Vec::new();
    let mut i = 0;
    while i < records.len() {
        let st = records[i].trim_matches(['\n', '\r']);
        i += 1;
        if st.is_empty() {
            continue;
        }
        let status = st.chars().next().unwrap_or('M').to_string();
        let path = if st.starts_with('R') || st.starts_with('C') {
            i += 1;
            let dst = records.get(i).copied().unwrap_or("");
            i += 1;
            dst.to_string()
        } else {
            let p = records.get(i).copied().unwrap_or("").to_string();
            i += 1;
            p
        };
        let path = path.trim_matches(['\n', '\r']).to_string();
        if path.is_empty() {
            continue;
        }
        changes.push(GitChange {
            status,
            path,
            staged: true,
            conflicted: false,
        });
    }
    Ok(changes)
}

pub fn show_diff(dir: &Path, hash: &str, file: &str) -> Result<String> {
    git(dir, &["show", "--format=", hash, "--", file])
}

pub fn show_file(dir: &Path, reference: &str, file: &str) -> Result<String> {
    git(dir, &["show", &format!("{reference}:{file}")])
}

pub fn list_tree(dir: &Path, reference: &str, subpath: &str) -> Vec<String> {
    git(
        dir,
        &[
            "-c",
            "core.quotePath=false",
            "ls-tree",
            "-r",
            "--name-only",
            reference,
            "--",
            subpath,
        ],
    )
    .unwrap_or_default()
    .lines()
    .map(|l| l.trim().to_string())
    .filter(|l| !l.is_empty())
    .collect()
}

pub fn name_status(
    dir: &Path,
    from: &str,
    to: &str,
    subpath: &str,
) -> Vec<(String, String)> {
    git(
        dir,
        &[
            "-c",
            "core.quotePath=false",
            "diff",
            "--name-status",
            from,
            to,
            "--",
            subpath,
        ],
    )
    .unwrap_or_default()
    .lines()
    .filter_map(|l| {
        let mut parts = l.splitn(2, '\t');
        let status = parts.next()?.chars().next()?.to_string();
        let rest = parts.next()?.trim();
        let path = rest.rsplit('\t').next().unwrap_or(rest).to_string();
        if path.is_empty() {
            None
        } else {
            Some((status, path))
        }
    })
    .collect()
}

pub fn name_status_working(dir: &Path, subpath: &str) -> Vec<(String, String)> {
    git(
        dir,
        &[
            "-c",
            "core.quotePath=false",
            "diff",
            "--name-status",
            "HEAD",
            "--",
            subpath,
        ],
    )
    .unwrap_or_default()
    .lines()
    .filter_map(|l| {
        let mut parts = l.splitn(2, '\t');
        let status = parts.next()?.chars().next()?.to_string();
        let rest = parts.next()?.trim();
        let path = rest.rsplit('\t').next().unwrap_or(rest).to_string();
        if path.is_empty() {
            None
        } else {
            Some((status, path))
        }
    })
    .collect()
}

pub fn diff_file(dir: &Path, file: &str, staged: bool) -> Result<String> {
    let args: Vec<&str> = if staged {
        vec!["diff", "--cached", "--", file]
    } else {
        vec!["diff", "HEAD", "--", file]
    };
    let out = git(dir, &args).unwrap_or_default();
    if out.trim().is_empty() {
        if let Ok(content) = std::fs::read_to_string(dir.join(file)) {
            let mut s = String::new();
            for line in content.lines() {
                s.push('+');
                s.push_str(line);
                s.push('\n');
            }
            return Ok(s);
        }
    }
    Ok(out)
}

pub fn discard_file(dir: &Path, file: &str) -> Result<()> {
    if has_head(dir) && git(dir, &["checkout", "HEAD", "--", file]).is_ok() {
        return Ok(());
    }
    let _ = git(dir, &["rm", "-f", "-q", "--", file]);
    let p = dir.join(file);
    if p.exists() {
        let _ = std::fs::remove_file(p);
    }
    Ok(())
}

pub fn revert_files(dir: &Path, files: &[String]) -> Result<()> {
    for f in files {
        discard_file(dir, f)?;
    }
    Ok(())
}

pub fn resolve_conflict(dir: &Path, file: &str, side: &str) -> Result<()> {
    let flag = if side == "theirs" {
        "--theirs"
    } else {
        "--ours"
    };
    git(dir, &["checkout", flag, "--", file])?;
    git(dir, &["add", "--", file])?;
    Ok(())
}

pub fn discard_all(dir: &Path) -> Result<()> {
    if has_head(dir) {
        git(dir, &["reset", "--hard", "HEAD"])?;
    }
    let _ = git(dir, &["clean", "-fd"]);
    Ok(())
}

pub fn revert_commit(dir: &Path, hash: &str) -> Result<String> {
    git(dir, &["revert", "--no-edit", hash])
}

pub fn reset(dir: &Path, hash: &str, mode: &str) -> Result<()> {
    let flag = match mode {
        "soft" => "--soft",
        "hard" => "--hard",
        _ => "--mixed",
    };
    git(dir, &["reset", flag, hash])?;
    Ok(())
}

pub fn cherry_pick(dir: &Path, hash: &str) -> Result<String> {
    git(dir, &["cherry-pick", hash])
}

pub fn stash_list(dir: &Path) -> Result<Vec<Stash>> {
    if !is_repo(dir) {
        return Ok(vec![]);
    }
    let out = git(dir, &["stash", "list", "--format=%gd%x1f%gs%x1f%cr"])
        .unwrap_or_default();
    let mut list = Vec::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.split('\u{1f}').collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let raw = parts.get(1).copied().unwrap_or("");
        let (branch, message) = match raw.split_once(": ") {
            Some((b, m)) => (
                b.trim_start_matches("On ")
                    .trim_start_matches("WIP on ")
                    .to_string(),
                m.to_string(),
            ),
            None => (String::new(), raw.to_string()),
        };
        list.push(Stash {
            reference: parts[0].to_string(),
            message,
            branch,
            relative: parts.get(2).copied().unwrap_or("").to_string(),
        });
    }
    Ok(list)
}

pub fn stash_push(
    dir: &Path,
    message: &str,
    include_untracked: bool,
) -> Result<String> {
    let mut args = vec!["stash", "push"];
    if include_untracked {
        args.push("-u");
    }
    if !message.trim().is_empty() {
        args.push("-m");
        args.push(message);
    }
    git(dir, &args)
}

pub fn stash_apply(dir: &Path, reference: &str, drop: bool) -> Result<String> {
    git(
        dir,
        &["stash", if drop { "pop" } else { "apply" }, reference],
    )
}

pub fn stash_drop(dir: &Path, reference: &str) -> Result<()> {
    git(dir, &["stash", "drop", reference])?;
    Ok(())
}

pub fn latest_tag(dir: &Path) -> Option<String> {
    let out = git(dir, &["describe", "--tags", "--abbrev=0"]).ok()?;
    let t = out.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

pub fn tags(dir: &Path) -> Result<Vec<Tag>> {
    if !is_repo(dir) {
        return Ok(vec![]);
    }
    let out = git(
        dir,
        &[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(refname:short)%00%(contents:subject)",
            "refs/tags",
        ],
    )
    .unwrap_or_default();
    let mut list = Vec::new();
    for line in out.lines() {
        let mut parts = line.split('\0');
        let name = parts.next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }
        list.push(Tag {
            name,
            subject: parts.next().unwrap_or("").to_string(),
        });
    }
    Ok(list)
}

pub fn create_tag(
    dir: &Path,
    name: &str,
    message: Option<&str>,
    target: Option<&str>,
) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        bail!("Enter a tag name");
    }
    let mut args = vec!["tag"];
    if let Some(m) = message.filter(|m| !m.trim().is_empty()) {
        args.push("-a");
        args.push(name);
        args.push("-m");
        args.push(m);
    } else {
        args.push(name);
    }
    if let Some(t) = target.filter(|t| !t.trim().is_empty()) {
        args.push(t);
    }
    git(dir, &args)?;
    Ok(())
}

pub fn delete_tag(dir: &Path, name: &str) -> Result<()> {
    git(dir, &["tag", "-d", name])?;
    Ok(())
}

pub fn push_tag(dir: &Path, name: &str) -> Result<String> {
    let remotes = remote_names(dir);
    let remote = remotes
        .iter()
        .find(|r| *r == "origin")
        .or_else(|| remotes.first())
        .ok_or_else(|| anyhow!("No remote to push to."))?;
    if name.trim().is_empty() {
        git_net(dir, &["push", remote, "--tags"])
    } else {
        git_net(dir, &["push", remote, "tag", name])
    }
}

pub fn remotes(dir: &Path) -> Result<Vec<Remote>> {
    if !is_repo(dir) {
        return Ok(vec![]);
    }
    let mut list = Vec::new();
    for name in remote_names(dir) {
        let url = git(dir, &["remote", "get-url", &name])
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        list.push(Remote { name, url });
    }
    Ok(list)
}

pub fn add_remote(dir: &Path, name: &str, url: &str) -> Result<()> {
    git(dir, &["remote", "add", name, url])?;
    Ok(())
}

pub fn set_remote_url(dir: &Path, name: &str, url: &str) -> Result<()> {
    git(dir, &["remote", "set-url", name, url])?;
    Ok(())
}

pub fn remove_remote(dir: &Path, name: &str) -> Result<()> {
    git(dir, &["remote", "remove", name])?;
    Ok(())
}

pub fn pack_remote_url(dir: &Path) -> Option<String> {
    if !is_repo(dir) {
        return None;
    }
    let remote = git(dir, &["remote", "get-url", "origin"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;
    let branch = git(dir, &["rev-parse", "--abbrev-ref", "origin/HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
        .and_then(|s| s.strip_prefix("origin/").map(|x| x.to_string()))
        .filter(|s| !s.is_empty())
        .or_else(|| current_branch(dir))
        .unwrap_or_else(|| "main".to_string());
    raw_host_url(&remote, &branch)
}

pub fn github_repo(dir: &Path) -> Option<String> {
    let remote = git(dir, &["remote", "get-url", "origin"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;
    let (host, path) = parse_remote(&remote)?;
    if !host.contains("github.com") {
        return None;
    }
    Some(path.trim_end_matches(".git").to_string())
}

fn raw_host_url(remote: &str, branch: &str) -> Option<String> {
    let (host, path) = parse_remote(remote)?;
    let path = path.trim_end_matches(".git");
    match host.as_str() {
        "github.com" => {
            Some(format!("https://raw.githubusercontent.com/{path}/{branch}"))
        }
        "gitlab.com" => {
            Some(format!("https://gitlab.com/{path}/-/raw/{branch}"))
        }
        _ => None,
    }
}

fn parse_remote(remote: &str) -> Option<(String, String)> {
    if let Some(rest) = remote.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        return Some((host.to_string(), path.to_string()));
    }
    let after = remote.split("://").nth(1).unwrap_or(remote);
    let after = after.trim_start_matches("git@");
    let (host, path) = after.split_once('/')?;
    Some((host.to_string(), path.to_string()))
}
