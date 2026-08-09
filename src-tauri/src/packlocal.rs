use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const LOCAL_FILE: &str = ".pack-local.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackLocal {
    #[serde(default)]
    pub instance_dir: Option<String>,
}

pub fn read(dir: &Path) -> PackLocal {
    let p = dir.join(LOCAL_FILE);
    std::fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn write(dir: &Path, local: &PackLocal) -> Result<()> {
    let p = dir.join(LOCAL_FILE);
    std::fs::write(p, format!("{}\n", serde_json::to_string_pretty(local)?))?;
    Ok(())
}
