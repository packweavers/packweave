use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

use crate::manifest::{self, Manifest};
use ts_rs::TS;

pub const DEFAULT_DATAPACKS: &str = "datapacks";

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize, TS,
)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "types.ts")]
pub enum ProjectType {
    #[default]
    Mod,
    Resourcepack,
    Shader,
    Datapack,
}

impl ProjectType {
    pub const ALL: [ProjectType; 4] = [
        ProjectType::Mod,
        ProjectType::Resourcepack,
        ProjectType::Shader,
        ProjectType::Datapack,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ProjectType::Mod => "mod",
            ProjectType::Resourcepack => "resourcepack",
            ProjectType::Shader => "shader",
            ProjectType::Datapack => "datapack",
        }
    }

    pub fn content_dir(self) -> &'static str {
        match self {
            ProjectType::Mod => "mods",
            ProjectType::Resourcepack => "resourcepacks",
            ProjectType::Shader => "shaders",
            ProjectType::Datapack => "datapacks",
        }
    }

    pub fn instance_dir(self) -> &'static str {
        match self {
            ProjectType::Mod => "mods",
            ProjectType::Resourcepack => "resourcepacks",
            ProjectType::Shader => "shaderpacks",
            ProjectType::Datapack => DEFAULT_DATAPACKS,
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            ProjectType::Datapack => &["zip"],
            _ => &["jar", "zip"],
        }
    }

    pub fn from_content_dir(sub: &str) -> Self {
        match sub {
            "resourcepacks" => ProjectType::Resourcepack,
            "shaders" => ProjectType::Shader,
            "datapacks" => ProjectType::Datapack,
            _ => ProjectType::Mod,
        }
    }

    pub fn from_content_path(path: &str) -> Self {
        if path.contains("/resourcepacks/") {
            ProjectType::Resourcepack
        } else if path.contains("/shaders/") {
            ProjectType::Shader
        } else if path.contains("/datapacks/") {
            ProjectType::Datapack
        } else {
            ProjectType::Mod
        }
    }

    pub fn from_api(raw: &str) -> Self {
        match raw {
            "resourcepack" => ProjectType::Resourcepack,
            "shader" => ProjectType::Shader,
            "datapack" => ProjectType::Datapack,
            _ => ProjectType::Mod,
        }
    }

    pub fn version_loader(self, loader: &str) -> Option<&str> {
        match self {
            ProjectType::Mod => Some(loader),
            ProjectType::Datapack => Some("datapack"),
            ProjectType::Resourcepack | ProjectType::Shader => None,
        }
    }
}

impl fmt::Display for ProjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Folders {
    datapacks: String,
}

impl Default for Folders {
    fn default() -> Self {
        Folders {
            datapacks: DEFAULT_DATAPACKS.to_string(),
        }
    }
}

impl Folders {
    pub fn of(manifest: &Manifest) -> Self {
        manifest
            .datapacks
            .as_deref()
            .and_then(datapack_folder)
            .map(|datapacks| Folders { datapacks })
            .unwrap_or_default()
    }

    pub fn load(pack_dir: &Path) -> Self {
        manifest::read(pack_dir)
            .map(|m| Folders::of(&m))
            .unwrap_or_default()
    }

    pub fn datapacks(&self) -> &str {
        &self.datapacks
    }

    pub fn install_dir(&self, project_type: ProjectType) -> &str {
        match project_type {
            ProjectType::Datapack => &self.datapacks,
            other => other.instance_dir(),
        }
    }
}

pub fn datapack_folder(raw: &str) -> Option<String> {
    let path = raw.trim().replace('\\', "/");
    if path.starts_with('/') || path.contains(':') {
        return None;
    }
    let parts: Vec<&str> = path
        .split('/')
        .map(str::trim)
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    if parts.is_empty() || parts.contains(&"..") {
        return None;
    }
    let taken = [
        ProjectType::Mod,
        ProjectType::Resourcepack,
        ProjectType::Shader,
    ];
    if taken.iter().any(|t| parts[0] == t.instance_dir()) {
        return None;
    }
    Some(parts.join("/"))
}
