use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;

pub const MANIFEST_FILE: &str = "modpack.json";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Manifest {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub minecraft: String,
    pub loader: String,
    #[serde(default)]
    pub loader_version: Option<String>,
    #[serde(default)]
    pub channel: Channel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub datapacks: Option<String>,
    #[serde(default = "enabled", skip_serializing_if = "is_enabled")]
    pub packweave_md: bool,
    #[serde(default, skip_serializing_if = "crate::util::is_zero")]
    pub format_version: u32,
}

fn enabled() -> bool {
    true
}

fn is_enabled(value: &bool) -> bool {
    *value
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "types.ts")]
pub enum Channel {
    #[default]
    Release,
    Beta,
    Alpha,
}

impl Channel {
    pub fn rank(self) -> u8 {
        match self {
            Channel::Release => 0,
            Channel::Beta => 1,
            Channel::Alpha => 2,
        }
    }
}

fn default_version() -> String {
    "1.0.0".into()
}

pub fn path_for(dir: &Path) -> PathBuf {
    dir.join(MANIFEST_FILE)
}

pub fn read(dir: &Path) -> Result<Manifest> {
    let p = path_for(dir);
    let text = std::fs::read_to_string(&p)
        .map_err(|e| anyhow::anyhow!("Could not read {}: {e}", p.display()))?;
    if let Ok(manifest) = serde_json::from_str::<Manifest>(&text) {
        return Ok(manifest);
    }
    let conflicted = crate::lenient::has_conflict(&text);
    let source = if conflicted {
        crate::lenient::ours(&text)
    } else {
        text
    };
    let (minecraft, loader) = crate::lockfile::cached_game(dir).unwrap_or_default();
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Modpack".into());
    let base = serde_json::json!({
        "name": name,
        "minecraft": minecraft,
        "loader": if loader.is_empty() { "vanilla".to_string() } else { loader },
    });
    let manifest = match crate::lenient::repair::<Manifest>(&source, base.clone()) {
        Some((manifest, _)) => manifest,
        None => serde_json::from_value(base)?,
    };
    if !conflicted && std::fs::write(dir.join(format!("{MANIFEST_FILE}.bak")), &source).is_ok() {
        let _ = write(dir, &manifest);
    }
    Ok(manifest)
}

pub fn write(dir: &Path, manifest: &Manifest) -> Result<()> {
    let p = path_for(dir);
    let text = serde_json::to_string_pretty(manifest)?;
    std::fs::write(&p, format!("{text}\n"))?;
    Ok(())
}
