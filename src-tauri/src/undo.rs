use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::content;
use crate::manifest;
use ts_rs::TS;

const HISTORY_LIMIT: usize = 40;
const SNAPSHOT_BYTE_LIMIT: usize = 256 * 1024 * 1024;

pub struct Snapshot {
    label: String,
    dirs: Vec<(PathBuf, BTreeMap<PathBuf, Vec<u8>>)>,
    files: Vec<(PathBuf, Option<Vec<u8>>)>,
    bytes: usize,
}

impl Snapshot {
    pub fn new(label: impl Into<String>) -> Self {
        Snapshot {
            label: label.into(),
            dirs: Vec::new(),
            files: Vec::new(),
            bytes: 0,
        }
    }

    pub fn pack(pack_dir: &Path, label: impl Into<String>) -> Self {
        Snapshot::new(label)
            .dir(&content::content_dir(pack_dir))
            .file(&manifest::path_for(pack_dir))
    }

    pub fn dir(mut self, dir: &Path) -> Self {
        let mut files = BTreeMap::new();
        collect(dir, dir, &mut files);
        self.bytes += files.values().map(Vec::len).sum::<usize>();
        self.dirs.push((dir.to_path_buf(), files));
        self
    }

    pub fn file(mut self, path: &Path) -> Self {
        if self.files.iter().any(|(p, _)| p == path) {
            return self;
        }
        let bytes = std::fs::read(path).ok();
        self.bytes += bytes.as_ref().map(Vec::len).unwrap_or(0);
        self.files.push((path.to_path_buf(), bytes));
        self
    }

    fn recapture(&self) -> Snapshot {
        let mut next = Snapshot::new(self.label.clone());
        for (dir, _) in &self.dirs {
            next = next.dir(dir);
        }
        for (path, _) in &self.files {
            next = next.file(path);
        }
        next
    }

    fn restore(&self) -> Result<()> {
        for (dir, files) in &self.dirs {
            let mut current = BTreeMap::new();
            collect(dir, dir, &mut current);
            for rel in current.keys() {
                if !files.contains_key(rel) {
                    crate::util::delete_file(&dir.join(rel))?;
                }
            }
            for (rel, bytes) in files {
                write(&dir.join(rel), bytes)?;
            }
            prune_empty(dir);
        }
        for (path, bytes) in &self.files {
            match bytes {
                Some(b) => write(path, b)?,
                None => crate::util::delete_file(path)?,
            }
        }
        Ok(())
    }
}

fn collect(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(base, &path, out);
        } else if let (Ok(rel), Ok(bytes)) = (path.strip_prefix(base), std::fs::read(&path)) {
            out.insert(rel.to_path_buf(), bytes);
        }
    }
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

fn prune_empty(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            prune_empty(&path);
            let _ = std::fs::remove_dir(&path);
        }
    }
}

#[derive(Default)]
struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

#[derive(Serialize, Default, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct UndoState {
    pub undo: Option<String>,
    pub redo: Option<String>,
}

fn histories() -> &'static Mutex<HashMap<PathBuf, History>> {
    static HISTORIES: OnceLock<Mutex<HashMap<PathBuf, History>>> = OnceLock::new();
    HISTORIES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn record(pack_dir: &Path, snapshot: Snapshot) {
    let Ok(mut all) = histories().lock() else {
        return;
    };
    let history = all.entry(pack_dir.to_path_buf()).or_default();
    history.redo.clear();
    if snapshot.bytes > SNAPSHOT_BYTE_LIMIT {
        history.undo.clear();
        return;
    }
    history.undo.push(snapshot);
    if history.undo.len() > HISTORY_LIMIT {
        history.undo.remove(0);
    }
}

pub fn forget(pack_dir: &Path) {
    if let Ok(mut all) = histories().lock() {
        all.remove(pack_dir);
    }
}

pub fn undo(pack_dir: &Path) -> Result<Option<String>> {
    step(pack_dir, true)
}

pub fn redo(pack_dir: &Path) -> Result<Option<String>> {
    step(pack_dir, false)
}

fn step(pack_dir: &Path, backwards: bool) -> Result<Option<String>> {
    let mut all = histories()
        .lock()
        .map_err(|_| anyhow::anyhow!("undo history is unavailable"))?;
    let history = all.entry(pack_dir.to_path_buf()).or_default();
    let taken = if backwards {
        history.undo.pop()
    } else {
        history.redo.pop()
    };
    let Some(snapshot) = taken else {
        return Ok(None);
    };
    let opposite = snapshot.recapture();
    snapshot.restore()?;
    let label = snapshot.label.clone();
    if backwards {
        history.redo.push(opposite);
    } else {
        history.undo.push(opposite);
    }
    Ok(Some(label))
}

pub fn state(pack_dir: &Path) -> UndoState {
    let Ok(all) = histories().lock() else {
        return UndoState::default();
    };
    match all.get(pack_dir) {
        Some(h) => UndoState {
            undo: h.undo.last().map(|s| s.label.clone()),
            redo: h.redo.last().map(|s| s.label.clone()),
        },
        None => UndoState::default(),
    }
}
