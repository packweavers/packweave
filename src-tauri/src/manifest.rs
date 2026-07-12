use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MANIFEST_FILE: &str = "modpack.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
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
    let manifest: Manifest = serde_json::from_str(&text).map_err(|e| {
        anyhow::anyhow!("{} is not a valid manifest: {e}", p.display())
    })?;
    Ok(manifest)
}

pub fn write(dir: &Path, manifest: &Manifest) -> Result<()> {
    let p = path_for(dir);
    let text = serde_json::to_string_pretty(manifest)?;
    std::fs::write(&p, format!("{text}\n"))?;
    Ok(())
}
