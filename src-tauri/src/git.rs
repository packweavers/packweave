use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::secrets;
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct GitStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub detached: bool,
    pub ahead: u32,
    pub behind: u32,
    pub has_remote: bool,
    pub clean: bool,
    pub conflicts: u32,
    pub changes: Vec<GitChange>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct GitChange {
    pub status: String,
    pub path: String,
    pub staged: bool,
    pub conflicted: bool,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct GitCommit {
    pub hash: String,
    pub short: String,
    pub subject: String,
    pub author: String,
    pub email: String,
    pub relative: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Branch {
    pub name: String,
    pub current: bool,
    pub remote: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub gone: bool,
    pub last: Option<String>,
    pub head: Option<String>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Branches {
    pub current: Option<String>,
    pub detached: bool,
    pub list: Vec<Branch>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Remote {
    pub name: String,
    pub url: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Stash {
    pub reference: String,
    pub message: String,
    pub branch: String,
    pub relative: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Tag {
    pub name: String,
    pub subject: String,
    pub commit: String,
}

pub const MISSING: &str = "GIT_MISSING:Git isn't installed.";
pub const NO_IDENTITY: &str = "GIT_IDENTITY:Set the name and email for your commits.";

static GIT_BIN: std::sync::RwLock<Option<Option<PathBuf>>> = std::sync::RwLock::new(None);

fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn runs(exe: &Path) -> bool {
    quiet(&mut Command::new(exe))
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn on_path() -> Vec<PathBuf> {
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD".into())
            .split(';')
            .filter(|e| !e.is_empty())
            .map(|e| e.to_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    let mut out = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for ext in &exts {
                let candidate = dir.join(format!("git{ext}"));
                if candidate.is_file() {
                    out.push(candidate);
                }
            }
        }
    }
    out
}

fn extra_locations() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        for base in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(dir) = std::env::var_os(base) {
                let dir = PathBuf::from(dir);
                v.push(dir.join("Git/cmd/git.exe"));
                v.push(dir.join("Programs/Git/cmd/git.exe"));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        v.push(PathBuf::from("/opt/homebrew/bin/git"));
        v.push(PathBuf::from("/usr/local/bin/git"));
    }

    #[cfg(target_os = "linux")]
    {
        v.push(PathBuf::from("/usr/bin/git"));
        v.push(PathBuf::from("/usr/local/bin/git"));
        v.push(PathBuf::from("/var/lib/flatpak/exports/bin/git"));
    }
    v
}

#[cfg(target_os = "macos")]
pub fn clt_installed() -> bool {
    quiet(&mut Command::new("/usr/bin/xcode-select"))
        .arg("-p")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn detect() -> Option<PathBuf> {
    let shim = Path::new("/usr/bin/git");
    for candidate in on_path().into_iter().chain(extra_locations()) {
        if cfg!(target_os = "macos") && candidate == shim {
            continue;
        }
        if candidate.is_file() && runs(&candidate) {
            return Some(candidate);
        }
    }

    #[cfg(target_os = "macos")]
    if clt_installed() && shim.is_file() {
        return Some(shim.to_path_buf());
    }
    None
}

pub fn binary() -> Option<PathBuf> {
    if let Some(cached) = GIT_BIN.read().ok().and_then(|g| g.clone()) {
        return cached;
    }
    let found = detect();
    if let Ok(mut slot) = GIT_BIN.write() {
        *slot = Some(found.clone());
    }
    found
}

pub fn forget_binary() {
    if let Ok(mut slot) = GIT_BIN.write() {
        *slot = None;
    }
}

pub fn installed() -> bool {
    binary().is_some()
}

fn new_git() -> Command {
    let exe = binary().unwrap_or_else(|| PathBuf::from("git"));
    let mut cmd = Command::new(exe);
    cmd.env("GIT_EDITOR", "true");
    quiet(&mut cmd);
    cmd
}

fn git_missing() -> Option<anyhow::Error> {
    if installed() {
        None
    } else {
        Some(anyhow!(MISSING))
    }
}

pub fn identity() -> (Option<String>, Option<String>) {
    let read = |key: &str| {
        if !installed() {
            return None;
        }
        let out = new_git().args(["config", "--global", key]).output().ok()?;
        if !out.status.success() {
            return None;
        }
        let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!v.is_empty()).then_some(v)
    };
    (read("user.name"), read("user.email"))
}

pub fn set_identity(name: &str, email: &str) -> Result<()> {
    let name = name.trim();
    let email = email.trim();
    if name.is_empty() || email.is_empty() {
        bail!("Enter a name and an email");
    }
    if let Some(err) = git_missing() {
        return Err(err);
    }
    for (key, value) in [("user.name", name), ("user.email", email)] {
        let out = new_git()
            .args(["config", "--global", key, value])
            .output()
            .map_err(|e| anyhow!("couldn't save your git settings: {e}"))?;
        if !out.status.success() {
            bail!(
                "couldn't save your git settings: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
    }
    Ok(())
}

fn has_identity(dir: &Path) -> bool {
    ["user.name", "user.email"].iter().all(|key| {
        new_git()
            .current_dir(dir)
            .args(["config", "--get", key])
            .output()
            .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty())
            .unwrap_or(false)
    })
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    if let Some(err) = git_missing() {
        return Err(err);
    }
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
    if git_missing().is_some() {
        return false;
    }
    new_git()
        .current_dir(dir)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn token_key(host: &str) -> String {
    format!("git_token:{}", host.to_lowercase())
}

pub fn token_for(host: &str) -> Option<String> {
    secrets::get(&token_key(host))
        .or_else(|| secrets::get("git_token"))
        .filter(|t| !t.trim().is_empty())
}

fn https_host(url: &str) -> Option<String> {
    let lower = url.trim().to_lowercase();
    if !lower.starts_with("https://") && !lower.starts_with("http://") {
        return None;
    }
    parse_remote(url).map(|(host, _)| host.to_lowercase())
}

fn auth_hosts(dir: Option<&Path>, url: Option<&str>) -> Vec<String> {
    let mut hosts: Vec<String> = Vec::new();
    if let Some(u) = url.and_then(https_host) {
        hosts.push(u);
    }
    if let Some(d) = dir {
        for name in remote_names(d) {
            let remote = git(d, &["remote", "get-url", &name]).unwrap_or_default();
            if let Some(h) = https_host(remote.trim()) {
                hosts.push(h);
            }
        }
    }
    hosts.sort();
    hosts.dedup();
    hosts
}

fn auth_user(host: &str) -> &'static str {
    if host.eq_ignore_ascii_case("github.com") {
        "x-access-token"
    } else {
        "oauth2"
    }
}

fn net_command(dir: Option<&Path>, url: Option<&str>) -> Command {
    use base64::Engine;
    let mut cmd = new_git();
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let mut authed = false;
    for host in auth_hosts(dir, url) {
        let Some(token) = token_for(&host) else {
            continue;
        };
        let auth = base64::engine::general_purpose::STANDARD.encode(format!(
            "{}:{}",
            auth_user(&host),
            token.trim()
        ));
        cmd.arg("-c").arg(format!(
            "http.https://{host}/.extraheader=Authorization: Basic {auth}"
        ));
        authed = true;
    }
    if authed {
        cmd.arg("-c").arg("credential.helper=");
        cmd.env("GCM_INTERACTIVE", "never");
    }
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

fn git_net(dir: &Path, args: &[&str]) -> Result<String> {
    if let Some(err) = git_missing() {
        return Err(err);
    }
    let mut cmd = net_command(Some(dir), None);
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
        return anyhow!("GIT_AUTH:Authentication required for this repository.");
    }
    anyhow!("git {}: {}", args.join(" "), stderr)
}

pub fn clone(url: &str, dest: &Path) -> Result<()> {
    if let Some(err) = git_missing() {
        return Err(err);
    }
    let mut cmd = net_command(None, Some(url));
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

fn git_input(dir: &Path, args: &[&str], input: Vec<u8>) -> Result<Vec<u8>> {
    if let Some(err) = git_missing() {
        return Err(err);
    }
    let mut child = new_git()
        .current_dir(dir)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("git is not installed or not on PATH: {e}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("git {}: no input", args.join(" ")))?;
    let writer = std::thread::spawn(move || {
        use std::io::Write;
        stdin.write_all(&input)
    });
    let output = child.wait_with_output()?;
    let _ = writer.join();
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(anyhow!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub fn staged_files(dir: &Path) -> Result<Vec<(String, String)>> {
    let out = git(
        dir,
        &["-c", "core.quotePath=false", "ls-files", "--stage", "-z"],
    )?;
    Ok(out
        .split('\0')
        .filter_map(|record| {
            let (meta, path) = record.split_once('\t')?;
            let mut parts = meta.split(' ');
            let mode = parts.next()?;
            let oid = parts.next()?;
            let stage = parts.next()?;
            (stage == "0" && mode.starts_with("100")).then(|| (path.to_string(), oid.to_string()))
        })
        .collect())
}

pub fn each_blob(dir: &Path, oids: &[String], mut visit: impl FnMut(usize, &[u8])) -> Result<()> {
    use std::io::{BufRead, Read};
    if oids.is_empty() {
        return Ok(());
    }
    if let Some(err) = git_missing() {
        return Err(err);
    }
    let mut child = new_git()
        .current_dir(dir)
        .args(["cat-file", "--batch"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("git is not installed or not on PATH: {e}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("git cat-file: no input"))?;
    let input: Vec<u8> = oids
        .iter()
        .flat_map(|o| format!("{o}\n").into_bytes())
        .collect();
    let writer = std::thread::spawn(move || {
        use std::io::Write;
        stdin.write_all(&input)
    });
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("git cat-file: no output"))?;
    let mut reader = std::io::BufReader::new(stdout);
    let mut header = Vec::new();
    let mut blob = Vec::new();
    let mut result = Ok(());
    for (i, oid) in oids.iter().enumerate() {
        header.clear();
        if let Err(e) = reader.read_until(b'\n', &mut header) {
            result = Err(anyhow!("git cat-file: {e}"));
            break;
        }
        let text = String::from_utf8_lossy(&header);
        let size = text
            .trim_end()
            .rsplit(' ')
            .next()
            .and_then(|s| s.parse::<usize>().ok());
        let Some(size) = size.filter(|_| !text.trim_end().ends_with(" missing")) else {
            result = Err(anyhow!("git cat-file: {oid} is missing"));
            break;
        };
        blob.resize(size, 0);
        let mut newline = [0u8; 1];
        if let Err(e) = reader
            .read_exact(&mut blob)
            .and_then(|_| reader.read_exact(&mut newline))
        {
            result = Err(anyhow!("git cat-file: {e}"));
            break;
        }
        visit(i, &blob);
    }
    drop(reader);
    if result.is_err() {
        let _ = child.kill();
    }
    let _ = writer.join();
    let _ = child.wait();
    result
}

pub fn stage_blob(dir: &Path, path: &str, bytes: &[u8]) -> Result<()> {
    let out = git_input(dir, &["hash-object", "-w", "--stdin"], bytes.to_vec())?;
    let oid = String::from_utf8_lossy(&out).trim().to_string();
    if oid.is_empty() {
        bail!("git hash-object: no object id");
    }
    git(
        dir,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("100644,{oid},{path}"),
        ],
    )?;
    Ok(())
}

pub fn tracks(dir: &Path, path: &str) -> bool {
    git_ok(dir, &["ls-files", "--error-unmatch", "--", path])
}

pub fn upstream_head(dir: &Path) -> Option<String> {
    git(dir, &["rev-parse", "--verify", "-q", "@{u}"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn mark_tag(dir: &Path, name: &str, commit: &str) -> Result<()> {
    let reference = format!("refs/tags/{}", name.trim());
    git(dir, &["check-ref-format", &reference])?;
    match git(
        dir,
        &[
            "rev-parse",
            "--verify",
            "-q",
            &format!("{reference}^{{commit}}"),
        ],
    ) {
        Ok(existing) if existing.trim() == commit => Ok(()),
        Ok(_) => bail!("{} already tags another commit.", name.trim()),
        Err(_) => {
            git(dir, &["update-ref", &reference, commit, ""])?;
            Ok(())
        }
    }
}

pub fn ignored(dir: &Path, path: &str) -> bool {
    git_ok(dir, &["check-ignore", "-q", "--", path])
}

pub fn reset_index_json(dir: &Path) {
    if !is_repo(dir) {
        return;
    }
    let path = crate::content::INDEX_FILE;
    if git_ok(dir, &["cat-file", "-e", &format!("HEAD:{path}")]) {
        let _ = git(dir, &["checkout", "HEAD", "--", path]);
    } else if !tracks(dir, path) && !ignored(dir, path) {
        let _ = std::fs::remove_file(dir.join(path));
    }
}

fn unmerged(dir: &Path, path: &str) -> bool {
    git(dir, &["ls-files", "-u", "--", path]).is_ok_and(|out| !out.trim().is_empty())
}

pub fn tracked_files(dir: &Path) -> Option<HashSet<String>> {
    if !is_repo(dir) {
        return None;
    }
    let out = git(dir, &["-c", "core.quotePath=false", "ls-files", "-z"]).ok()?;
    Some(
        out.split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
    )
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

fn head_commit(dir: &Path) -> Option<String> {
    git(dir, &["rev-parse", "--verify", "-q", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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
            head: None,
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
        if path.is_empty() || path == crate::content::INDEX_FILE {
            continue;
        }
        let conflicted = x == "U" || y == "U" || (x == "A" && y == "A") || (x == "D" && y == "D");
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
    let head = head_commit(dir);
    let detached = branch.is_none() && head.is_some();
    let has_remote = !remote_names(dir).is_empty();
    let clean = changes.is_empty();

    Ok(GitStatus {
        is_repo: true,
        branch,
        head,
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

    let fmt = "%(HEAD)%00%(refname)%00%(refname:short)%00%(upstream:short)%00%(upstream:track)%00%(committerdate:relative)%00%(objectname:short)";
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
        let non_empty = |i: usize| {
            parts
                .get(i)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        list.push(Branch {
            name: short.clone(),
            current: !remote && current.as_deref() == Some(short.as_str()),
            remote,
            upstream,
            ahead: parse_count(track, "ahead"),
            behind: parse_count(track, "behind"),
            gone: track.contains("gone"),
            last: non_empty(5),
            head: non_empty(6),
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
                    last: None,
                    head: None,
                },
            );
        }
    }
    list.sort_by(|a, b| a.remote.cmp(&b.remote).then_with(|| a.name.cmp(&b.name)));

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

pub fn rebase_branch(dir: &Path, name: &str) -> Result<String> {
    git(dir, &["rebase", name])
}

pub fn commit(dir: &Path, message: &str, files: &[String], amend: bool) -> Result<()> {
    if message.trim().is_empty() {
        bail!("Enter a commit message");
    }
    if !amend && files.is_empty() {
        bail!("Select at least one change to commit");
    }
    if let Some(err) = git_missing() {
        return Err(err);
    }
    if !has_identity(dir) {
        bail!(NO_IDENTITY);
    }
    if mid_operation(dir) {
        crate::content::stage_index(dir)?;
        let args: &[&str] = if amend {
            &["commit", "--amend", "-m", message]
        } else {
            &["commit", "-m", message]
        };
        git(dir, args)?;
        return Ok(());
    }
    if has_head(dir) {
        git(dir, &["reset", "-q"])?;
    } else {
        git(
            dir,
            &["rm", "-r", "--cached", "--ignore-unmatch", "-q", "--", "."],
        )?;
    }
    if !files.is_empty() {
        let mut add = vec!["add", "-A", "--"];
        for f in files {
            add.push(f.as_str());
        }
        git(dir, &add)?;
    }
    crate::content::stage_index(dir)?;
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
    let ordered: Vec<&String> = std::iter::once(remotes.iter().find(|r| *r == "origin"))
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

pub fn push_branch(dir: &Path, remote: &str, branch: &str, set_upstream: bool) -> Result<String> {
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
    let out = git(dir, &["show", "--name-status", "-z", "--format=", hash]).unwrap_or_default();
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

pub fn name_status(dir: &Path, from: &str, to: &str, subpath: &str) -> Vec<(String, String)> {
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
    git(dir, &["rm", "-f", "--ignore-unmatch", "-q", "--", file])?;
    crate::util::delete_file(&dir.join(file))?;
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
    let out = git(dir, &["stash", "list", "--format=%gd%x1f%gs%x1f%cr"]).unwrap_or_default();
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

pub fn stash_push(dir: &Path, message: &str, include_untracked: bool) -> Result<String> {
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

pub fn credential_token(dir: &Path) -> Option<String> {
    if git_missing().is_some() {
        return None;
    }
    let mut cmd = new_git();
    cmd.current_dir(dir)
        .args(["credential", "fill"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = cmd.spawn().ok()?;
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut()?;
        stdin
            .write_all(b"protocol=https\nhost=github.com\n\n")
            .ok()?;
    }
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.strip_prefix("password=").map(|p| p.trim().to_string()))
        .filter(|p| !p.is_empty())
}

pub fn github_token(dir: &Path) -> Option<String> {
    token_for("github.com").or_else(|| credential_token(dir))
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

pub fn previous_tag(dir: &Path, tag: &str) -> Option<String> {
    let before = format!("{tag}^");
    git(dir, &["describe", "--tags", "--abbrev=0", &before])
        .ok()
        .map(|out| out.trim().to_string())
        .filter(|t| !t.is_empty())
        .or_else(|| latest_tag(dir).filter(|t| t != tag))
}

pub fn changed_paths_since(dir: &Path, reference: &str, prefix: &str) -> Result<Vec<String>> {
    let out = git(dir, &["diff", "--name-only", reference, "--", prefix])?;
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
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
            "--format=%(refname:short)%00%(contents:subject)%00%(objectname)%00%(*objectname)",
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
        let subject = parts.next().unwrap_or("").to_string();
        let object = parts.next().unwrap_or("");
        let peeled = parts.next().unwrap_or("");
        let commit = if peeled.is_empty() { object } else { peeled };
        list.push(Tag {
            name,
            subject,
            commit: commit.to_string(),
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
    let message = message.filter(|m| !m.trim().is_empty()).unwrap_or(name);
    let mut args = vec!["tag", "-a", name, "-m", message];
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
        "github.com" => Some(format!("https://raw.githubusercontent.com/{path}/{branch}")),
        "gitlab.com" => Some(format!("https://gitlab.com/{path}/-/raw/{branch}")),
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

pub fn run(dir: &Path, args: &[&str]) -> Result<String> {
    git(dir, args)
}

pub fn branch_exists(dir: &Path, name: &str) -> bool {
    ref_exists(dir, &format!("refs/heads/{name}"))
}

pub fn split_remote_branch(dir: &Path, name: &str) -> Option<String> {
    if !ref_exists(dir, &format!("refs/remotes/{name}")) {
        return None;
    }
    local_name_of_remote(dir, name)
}

pub fn remote_tracking_for(dir: &Path, name: &str) -> Option<String> {
    let remotes = remote_names(dir);
    let ordered = remotes
        .iter()
        .filter(|r| *r == "origin")
        .chain(remotes.iter().filter(|r| *r != "origin"));
    for remote in ordered {
        let candidate = format!("{remote}/{name}");
        if ref_exists(dir, &format!("refs/remotes/{candidate}")) {
            return Some(candidate);
        }
    }
    None
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct GraphCommit {
    pub hash: String,
    pub short: String,
    pub parents: Vec<String>,
    pub subject: String,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub relative: String,
    pub refs: Vec<String>,
}

const GRAPH_FORMAT: &str = "--format=%H%x1f%h%x1f%P%x1f%s%x1f%an%x1f%ae%x1f%ct%x1f%cr%x1f%D%x1e";

fn parse_graph(out: &str) -> Vec<GraphCommit> {
    let mut commits = Vec::new();
    for record in out.split('\u{1e}') {
        let record = record.trim_matches(['\n', '\r']);
        if record.is_empty() {
            continue;
        }
        let parts: Vec<&str> = record.split('\u{1f}').collect();
        if parts.len() < 9 {
            continue;
        }
        commits.push(GraphCommit {
            hash: parts[0].into(),
            short: parts[1].into(),
            parents: parts[2].split_whitespace().map(str::to_string).collect(),
            subject: parts[3].into(),
            author: parts[4].into(),
            email: parts[5].into(),
            time: parts[6].trim().parse().unwrap_or(0),
            relative: parts[7].into(),
            refs: parts[8]
                .split(", ")
                .map(|r| r.trim().to_string())
                .filter(|r| !r.is_empty())
                .collect(),
        });
    }
    commits
}

pub fn graph(dir: &Path, limit: u32, skip: u32, all: bool) -> Result<Vec<GraphCommit>> {
    if !is_repo(dir) || !has_head(dir) {
        return Ok(vec![]);
    }
    let count = format!("-n{limit}");
    let skip = format!("--skip={skip}");
    let mut args = vec!["log", "--date-order", &count, &skip, GRAPH_FORMAT];
    if all {
        args.push("--exclude=refs/stash");
        args.push("--all");
    }
    Ok(parse_graph(&git(dir, &args).unwrap_or_default()))
}

pub fn commits_between(dir: &Path, base: &str, other: &str) -> Result<Vec<GraphCommit>> {
    let range = format!("{base}..{other}");
    Ok(parse_graph(&git(dir, &["log", &range, GRAPH_FORMAT])?))
}

pub fn merge_base(dir: &Path, a: &str, b: &str) -> Option<String> {
    git(dir, &["merge-base", a, b])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct FileStat {
    pub path: String,
    pub status: String,
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

fn parse_numstat(out: &str) -> std::collections::HashMap<String, (Option<u32>, Option<u32>)> {
    let mut map = std::collections::HashMap::new();
    for line in out.lines() {
        let mut parts = line.splitn(3, '\t');
        let (Some(a), Some(r), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let path = path
            .rsplit(" => ")
            .next()
            .unwrap_or(path)
            .trim_end_matches('}');
        map.insert(path.to_string(), (a.parse().ok(), r.parse().ok()));
    }
    map
}

pub fn diff_stats(dir: &Path, from: &str, to: &str) -> Result<Vec<FileStat>> {
    let range = format!("{from}...{to}");
    let names = git(
        dir,
        &[
            "-c",
            "core.quotePath=false",
            "diff",
            "--name-status",
            &range,
        ],
    )?;
    let stats = parse_numstat(&git(
        dir,
        &["-c", "core.quotePath=false", "diff", "--numstat", &range],
    )?);
    let mut out = Vec::new();
    for line in names.lines() {
        let mut parts = line.split('\t');
        let Some(status) = parts.next().and_then(|s| s.chars().next()) else {
            continue;
        };
        let Some(path) = parts.next_back() else {
            continue;
        };
        let (added, removed) = stats.get(path).copied().unwrap_or((None, None));
        out.push(FileStat {
            path: path.to_string(),
            status: status.to_string(),
            added,
            removed,
        });
    }
    Ok(out)
}

pub fn working_stats(dir: &Path) -> std::collections::HashMap<String, (Option<u32>, Option<u32>)> {
    if !has_head(dir) {
        return std::collections::HashMap::new();
    }
    let mut stats = parse_numstat(
        &git(
            dir,
            &["-c", "core.quotePath=false", "diff", "--numstat", "HEAD"],
        )
        .unwrap_or_default(),
    );
    let untracked = git(
        dir,
        &[
            "-c",
            "core.quotePath=false",
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )
    .unwrap_or_default();
    for path in untracked.split('\0').filter(|p| !p.is_empty()) {
        let lines = std::fs::read(dir.join(path))
            .ok()
            .filter(|b| !b.iter().take(8000).any(|c| *c == 0))
            .map(|b| b.iter().filter(|c| **c == b'\n').count() as u32);
        stats.insert(path.to_string(), (lines, lines.map(|_| 0)));
    }
    stats
}

pub fn merge_preview(dir: &Path, base: &str, other: &str) -> Option<Vec<String>> {
    if git_missing().is_some() {
        return None;
    }
    let output = new_git()
        .current_dir(dir)
        .args([
            "merge-tree",
            "--write-tree",
            "--name-only",
            "--no-messages",
            base,
            other,
        ])
        .output()
        .ok()?;
    match output.status.code() {
        Some(0) => Some(Vec::new()),
        Some(1) => Some(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .skip(1)
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect(),
        ),
        _ => None,
    }
}

#[derive(Serialize, serde::Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct MergeNote {
    pub from: String,
    pub mode: String,
    pub message: String,
}

fn merge_note_path(dir: &Path) -> Option<PathBuf> {
    let out = git(dir, &["rev-parse", "--git-path", "packweave-merge.json"]).ok()?;
    let p = PathBuf::from(out.trim());
    Some(if p.is_absolute() { p } else { dir.join(p) })
}

pub fn conflicted_files(dir: &Path) -> Vec<String> {
    git(
        dir,
        &[
            "-c",
            "core.quotePath=false",
            "diff",
            "--name-only",
            "--diff-filter=U",
        ],
    )
    .unwrap_or_default()
    .lines()
    .map(|l| l.trim().to_string())
    .filter(|l| !l.is_empty() && l != crate::content::INDEX_FILE)
    .collect()
}

fn merging(dir: &Path) -> bool {
    git_ok(dir, &["rev-parse", "--verify", "-q", "MERGE_HEAD"])
        || merge_note_path(dir).is_some_and(|p| p.is_file())
}

pub fn merge(dir: &Path, from: &str, mode: &str, message: &str) -> Result<bool> {
    if let Some(err) = git_missing() {
        return Err(err);
    }
    if !has_identity(dir) {
        bail!(NO_IDENTITY);
    }
    let message = if message.trim().is_empty() {
        format!("Merge {from}")
    } else {
        message.trim().to_string()
    };
    let note = MergeNote {
        from: from.to_string(),
        mode: mode.to_string(),
        message: message.clone(),
    };
    let args: Vec<&str> = match mode {
        "squash" => vec!["merge", "--squash", from],
        "ff" => vec!["merge", "--ff-only", from],
        _ => vec!["merge", "--no-ff", "--no-commit", from],
    };
    let result = git(dir, &args);
    let conflicts = conflicted_files(dir);
    if !conflicts.is_empty() {
        if let Some(path) = merge_note_path(dir) {
            std::fs::write(path, serde_json::to_string(&note)?)?;
        }
        return Ok(true);
    }
    if mode == "ff" || !unmerged(dir, crate::content::INDEX_FILE) {
        result?;
    }
    let pending = match mode {
        "ff" => false,
        "squash" => !git_ok(dir, &["diff", "--cached", "--quiet"]),
        _ => git_ok(dir, &["rev-parse", "--verify", "-q", "MERGE_HEAD"]),
    };
    if pending {
        crate::content::stage_index(dir)?;
        git(dir, &["commit", "-m", &message])?;
    }
    Ok(false)
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct MergeState {
    pub merging: bool,
    pub note: Option<MergeNote>,
    pub conflicts: Vec<String>,
}

pub fn merge_state(dir: &Path) -> MergeState {
    if !is_repo(dir) {
        return MergeState {
            merging: false,
            note: None,
            conflicts: vec![],
        };
    }
    let note = merge_note_path(dir)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<MergeNote>(&t).ok());
    MergeState {
        merging: merging(dir),
        note,
        conflicts: conflicted_files(dir),
    }
}

pub fn conflict_side(dir: &Path, path: &str, stage: u8) -> Option<String> {
    git(dir, &["show", &format!(":{stage}:{path}")]).ok()
}

pub fn resolve_with(dir: &Path, path: &str, text: &str) -> Result<()> {
    let target = dir.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, text)?;
    git(dir, &["add", "--", path])?;
    Ok(())
}

pub fn merge_abort(dir: &Path) -> Result<()> {
    if git_ok(dir, &["rev-parse", "--verify", "-q", "MERGE_HEAD"]) {
        git(dir, &["merge", "--abort"])?;
    } else {
        git(dir, &["reset", "--merge"])?;
    }
    if let Some(path) = merge_note_path(dir) {
        crate::util::delete_file(&path)
            .map_err(|e| anyhow!("Couldn't remove {}: {e}", path.display()))?;
    }
    Ok(())
}

pub fn merge_commit(dir: &Path, message: &str) -> Result<()> {
    if !conflicted_files(dir).is_empty() {
        bail!("Settle every conflict first.");
    }
    if !has_identity(dir) {
        bail!(NO_IDENTITY);
    }
    let note_path = merge_note_path(dir);
    let message = if message.trim().is_empty() {
        note_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str::<MergeNote>(&t).ok())
            .map(|n| n.message)
            .unwrap_or_else(|| "Merge".into())
    } else {
        message.trim().to_string()
    };
    crate::content::stage_index(dir)?;
    git(dir, &["commit", "-m", &message])?;
    if let Some(path) = note_path {
        crate::util::delete_file(&path).map_err(|e| {
            anyhow!(
                "Committed the merge, but couldn't remove {}: {e}",
                path.display()
            )
        })?;
    }
    Ok(())
}

pub fn rev_count(dir: &Path, range: &str) -> u32 {
    git(dir, &["rev-list", "--count", range])
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}
