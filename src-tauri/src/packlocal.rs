use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const LOCAL_FILE: &str = ".pack-local.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackLocal {
    #[serde(default)]
    pub instance_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unchecked: Vec<String>,
}

pub fn read(dir: &Path) -> PackLocal {
    let Ok(text) = std::fs::read_to_string(dir.join(LOCAL_FILE)) else {
        return PackLocal::default();
    };
    if let Ok(local) = serde_json::from_str(&text) {
        return local;
    }
    let base = serde_json::to_value(PackLocal::default()).unwrap_or_default();
    match crate::lenient::repair::<PackLocal>(&text, base) {
        Some((local, _)) => {
            let _ = write(dir, &local);
            local
        }
        None => PackLocal::default(),
    }
}

pub fn write(dir: &Path, local: &PackLocal) -> Result<()> {
    let p = dir.join(LOCAL_FILE);
    std::fs::write(p, format!("{}\n", serde_json::to_string_pretty(local)?))?;
    Ok(())
}
