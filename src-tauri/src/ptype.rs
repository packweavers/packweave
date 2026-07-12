use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    #[default]
    Mod,
    Resourcepack,
    Shader,
}

impl ProjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectType::Mod => "mod",
            ProjectType::Resourcepack => "resourcepack",
            ProjectType::Shader => "shader",
        }
    }

    pub fn content_dir(self) -> &'static str {
        match self {
            ProjectType::Mod => "mods",
            ProjectType::Resourcepack => "resourcepacks",
            ProjectType::Shader => "shaders",
        }
    }

    pub fn instance_dir(self) -> &'static str {
        match self {
            ProjectType::Mod => "mods",
            ProjectType::Resourcepack => "resourcepacks",
            ProjectType::Shader => "shaderpacks",
        }
    }

    pub fn from_content_dir(sub: &str) -> Self {
        match sub {
            "resourcepacks" => ProjectType::Resourcepack,
            "shaders" => ProjectType::Shader,
            _ => ProjectType::Mod,
        }
    }

    pub fn from_content_path(path: &str) -> Self {
        if path.contains("/resourcepacks/") {
            ProjectType::Resourcepack
        } else if path.contains("/shaders/") {
            ProjectType::Shader
        } else {
            ProjectType::Mod
        }
    }

    pub fn from_instance_path(rel: &str) -> Self {
        if rel.starts_with("resourcepacks/") {
            ProjectType::Resourcepack
        } else if rel.starts_with("shaderpacks/") {
            ProjectType::Shader
        } else {
            ProjectType::Mod
        }
    }

    pub fn from_api(raw: &str) -> Self {
        match raw {
            "resourcepack" => ProjectType::Resourcepack,
            "shader" => ProjectType::Shader,
            _ => ProjectType::Mod,
        }
    }
}

impl fmt::Display for ProjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_format_is_lowercase() {
        assert_eq!(
            serde_json::to_string(&ProjectType::Shader).unwrap(),
            "\"shader\""
        );
        let p: ProjectType = serde_json::from_str("\"resourcepack\"").unwrap();
        assert_eq!(p, ProjectType::Resourcepack);
    }

    #[test]
    fn dirs_split_by_convention() {
        assert_eq!(ProjectType::Shader.content_dir(), "shaders");
        assert_eq!(ProjectType::Shader.instance_dir(), "shaderpacks");
        assert_eq!(
            ProjectType::from_content_dir("shaders"),
            ProjectType::Shader
        );
        assert_eq!(
            ProjectType::from_instance_path("shaderpacks/x.zip"),
            ProjectType::Shader
        );
        assert_eq!(
            ProjectType::from_content_path("content/mods/x.json"),
            ProjectType::Mod
        );
    }
}
