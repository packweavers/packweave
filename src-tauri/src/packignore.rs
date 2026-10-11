use anyhow::Result;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::content;
use crate::git;
use crate::instance;
use crate::lockfile::LockedMod;
use ts_rs::TS;

pub const PACKWEAVEIGNORE_FILE: &str = ".packweaveignore";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "types.ts")]
pub enum IgnoreKind {
    Git,
    Export,
}

const EXPORT_TEMPLATE: &str = "\
# pack documents
/README.md
/CHANGELOG.md
/PACKWEAVE.md

# system files
._*
Thumbs.db*
.DS_Store
.AppleDouble
.LSOverride
.Spotlight-V100
.Trashes
.fseventsd
.DocumentRevisions-V100
.TemporaryItems
.apdisk
ehthumbs.db
ehthumbs_vista.db
desktop.ini
.directory
$RECYCLE.BIN/
System Volume Information/
";

impl IgnoreKind {
    pub fn file_name(self) -> &'static str {
        match self {
            IgnoreKind::Git => instance::GITIGNORE_FILE,
            IgnoreKind::Export => PACKWEAVEIGNORE_FILE,
        }
    }

    pub fn defaults(self) -> &'static str {
        match self {
            IgnoreKind::Git => instance::DEFAULT_GITIGNORE,
            IgnoreKind::Export => EXPORT_TEMPLATE,
        }
    }
}

pub fn read(pack_dir: &Path, kind: IgnoreKind) -> String {
    match std::fs::read_to_string(pack_dir.join(kind.file_name())) {
        Ok(text) => text,
        Err(_) if kind == IgnoreKind::Export => kind.defaults().to_string(),
        Err(_) => String::new(),
    }
}

pub fn write(pack_dir: &Path, kind: IgnoreKind, text: &str) -> Result<()> {
    let path = pack_dir.join(kind.file_name());
    let text = if text.is_empty() || text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    };
    std::fs::write(path, text)?;
    Ok(())
}

pub fn add_pattern(pack_dir: &Path, kind: IgnoreKind, pattern: &str) -> Result<()> {
    let mut text = match std::fs::read_to_string(pack_dir.join(kind.file_name())) {
        Ok(text) => text,
        Err(_) => kind.defaults().to_string(),
    };
    if text.lines().any(|l| l.trim() == pattern) {
        return Ok(());
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(pattern);
    text.push('\n');
    write(pack_dir, kind, &text)
}

pub fn matcher(pack_dir: &Path, kind: IgnoreKind) -> Gitignore {
    let mut builder = GitignoreBuilder::new(pack_dir);
    let file = pack_dir.join(kind.file_name());
    if file.is_file() {
        let _ = builder.add(file);
    } else if kind == IgnoreKind::Export {
        for line in EXPORT_TEMPLATE.lines() {
            let _ = builder.add_line(None, line);
        }
    }
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

pub fn adopt_export_defaults(pack_dir: &Path) -> Result<()> {
    let path = pack_dir.join(PACKWEAVEIGNORE_FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let have: HashSet<&str> = text.lines().map(str::trim).collect();
    let missing: Vec<&str> = EXPORT_TEMPLATE
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !have.contains(l))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let mut next = text.clone();
    if !next.ends_with('\n') {
        next.push('\n');
    }
    for line in missing {
        next.push_str(line);
        next.push('\n');
    }
    crate::util::write_file(&path, next)
}

pub(crate) fn is_document(name: &str) -> bool {
    name.to_lowercase().ends_with(".md")
}

pub fn ignores(matcher: &Gitignore, rel: &str) -> bool {
    matcher
        .matched_path_or_any_parents(Path::new(rel), false)
        .is_ignore()
}

pub struct PackFile {
    pub rel: String,
    pub path: PathBuf,
}

pub struct ExportFilter {
    tracked: Option<HashSet<String>>,
    git: Gitignore,
    export: Gitignore,
}

impl ExportFilter {
    pub fn load(pack_dir: &Path) -> Self {
        ExportFilter {
            tracked: git::tracked_files(pack_dir),
            git: matcher(pack_dir, IgnoreKind::Git),
            export: matcher(pack_dir, IgnoreKind::Export),
        }
    }

    pub fn committed(pack_dir: &Path, tracked: HashSet<String>, export: Option<&str>) -> Self {
        let mut builder = GitignoreBuilder::new(pack_dir);
        for line in export.unwrap_or(EXPORT_TEMPLATE).lines() {
            let _ = builder.add_line(None, line);
        }
        ExportFilter {
            tracked: Some(tracked),
            git: Gitignore::empty(),
            export: builder.build().unwrap_or_else(|_| Gitignore::empty()),
        }
    }

    pub fn skips(&self, pack_rel: &str) -> bool {
        if pack_rel == crate::dist::PACK_JAR || pack_rel == crate::dist::PACK_MOD_JAR {
            return true;
        }
        let tracked = self
            .tracked
            .as_ref()
            .is_some_and(|set| set.contains(pack_rel));
        (!tracked && ignores(&self.git, pack_rel)) || ignores(&self.export, pack_rel)
    }

    pub fn skips_override(&self, rel: &str) -> bool {
        self.skips(&format!("{}/{rel}", instance::OVERRIDES_DIR)) || ignores(&self.export, rel)
    }

    pub fn skips_mod(&self, m: &LockedMod) -> bool {
        self.skips(&content::locked_rel_path(m))
    }

    pub fn overrides(&self, pack_dir: &Path) -> Vec<PackFile> {
        let base = instance::overrides_dir(pack_dir);
        let mut out = Vec::new();
        walk(&base, &base, &mut out);
        out.retain(|f| !self.skips_override(&f.rel));
        out.extend(self.documents(pack_dir));
        out.sort_by(|a, b| a.rel.cmp(&b.rel));
        out
    }

    pub fn tree(&self, pack_dir: &Path, sub: &str) -> Vec<PackFile> {
        let mut out = Vec::new();
        walk(pack_dir, &pack_dir.join(sub), &mut out);
        out.retain(|f| !self.skips(&f.rel));
        out.sort_by(|a, b| a.rel.cmp(&b.rel));
        out
    }

    pub fn documents(&self, pack_dir: &Path) -> Vec<PackFile> {
        let Ok(entries) = std::fs::read_dir(pack_dir) else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter(|e| e.path().is_file())
            .filter_map(|e| {
                let rel = e.file_name().to_string_lossy().to_string();
                (is_document(&rel) && !self.skips(&rel)).then(|| PackFile {
                    rel,
                    path: e.path(),
                })
            })
            .collect()
    }
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<PackFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(base, &path, out);
        } else if path.is_file() {
            if let Ok(rel) = path.strip_prefix(base) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                out.push(PackFile { rel, path });
            }
        }
    }
}
