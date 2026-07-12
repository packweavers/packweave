use serde::Serialize;
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDep {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<ModDep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minecraft: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack_format: Option<i64>,
}

fn is_env(id: &str) -> bool {
    matches!(
        id.to_lowercase().as_str(),
        "minecraft"
            | "java"
            | "fabricloader"
            | "fabric-loader"
            | "fabric_loader"
            | "quilt_loader"
            | "quiltloader"
            | "forge"
            | "neoforge"
    )
}

pub fn read_archive(path: &Path) -> Option<ModMeta> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let names: HashSet<String> = zip.file_names().map(String::from).collect();

    if names.contains("fabric.mod.json") {
        if let Some(m) =
            entry(&mut zip, "fabric.mod.json").and_then(|s| parse_fabric(&s))
        {
            return Some(cleanup(m));
        }
    }
    if names.contains("quilt.mod.json") {
        if let Some(m) =
            entry(&mut zip, "quilt.mod.json").and_then(|s| parse_quilt(&s))
        {
            return Some(cleanup(m));
        }
    }
    let manifest_ver = if names.contains("META-INF/MANIFEST.MF") {
        entry(&mut zip, "META-INF/MANIFEST.MF")
            .and_then(|s| manifest_version(&s))
    } else {
        None
    };
    if names.contains("META-INF/neoforge.mods.toml") {
        if let Some(m) = entry(&mut zip, "META-INF/neoforge.mods.toml")
            .and_then(|s| {
                parse_forge_toml(&s, "neoforge", manifest_ver.as_deref())
            })
        {
            return Some(cleanup(m));
        }
    }
    if names.contains("META-INF/mods.toml") {
        if let Some(m) = entry(&mut zip, "META-INF/mods.toml").and_then(|s| {
            parse_forge_toml(&s, "forge", manifest_ver.as_deref())
        }) {
            return Some(cleanup(m));
        }
    }
    if names.contains("pack.mcmeta") {
        if let Some(m) =
            entry(&mut zip, "pack.mcmeta").and_then(|s| parse_mcmeta(&s))
        {
            return Some(cleanup(m));
        }
    }
    if names.iter().any(|n| n.starts_with("shaders/")) {
        return Some(ModMeta {
            loaders: vec!["iris".into()],
            ..Default::default()
        });
    }
    None
}

fn entry(
    zip: &mut zip::ZipArchive<std::fs::File>,
    name: &str,
) -> Option<String> {
    let mut f = zip.by_name(name).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    Some(s)
}

fn manifest_version(s: &str) -> Option<String> {
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("Implementation-Version:") {
            let v = rest.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn parse_fabric(s: &str) -> Option<ModMeta> {
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    let mut m = ModMeta {
        loaders: vec!["fabric".into()],
        ..Default::default()
    };
    m.id = str_of(v.get("id"));
    m.name = str_of(v.get("name"));
    m.version = str_of(v.get("version"));
    m.description = str_of(v.get("description"));
    m.homepage = str_of(v.get("contact").and_then(|c| c.get("homepage")));
    if let Some(a) = v.get("authors").and_then(|x| x.as_array()) {
        for au in a {
            if let Some(s) = au.as_str() {
                m.authors.push(s.to_string());
            } else if let Some(n) = str_of(au.get("name")) {
                m.authors.push(n);
            }
        }
    }
    fabric_deps(&v, "depends", "required", &mut m);
    fabric_deps(&v, "recommends", "optional", &mut m);
    fabric_deps(&v, "suggests", "optional", &mut m);
    fabric_deps(&v, "breaks", "incompatible", &mut m);
    fabric_deps(&v, "conflicts", "incompatible", &mut m);
    Some(m)
}

fn fabric_deps(
    v: &serde_json::Value,
    field: &str,
    kind: &str,
    m: &mut ModMeta,
) {
    let Some(obj) = v.get(field).and_then(|x| x.as_object()) else {
        return;
    };
    for (id, req) in obj {
        let version = match req {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Array(a) => Some(
                a.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(" || "),
            ),
            _ => None,
        }
        .filter(|s| !s.is_empty() && s != "*");
        if id.eq_ignore_ascii_case("minecraft") {
            if m.minecraft.is_none() {
                m.minecraft = version;
            }
            continue;
        }
        if is_env(id) {
            continue;
        }
        m.dependencies.push(ModDep {
            id: id.clone(),
            version,
            kind: kind.into(),
        });
    }
}

fn parse_quilt(s: &str) -> Option<ModMeta> {
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    let ql = v.get("quilt_loader")?;
    let mut m = ModMeta {
        loaders: vec!["quilt".into()],
        ..Default::default()
    };
    m.id = str_of(ql.get("id"));
    m.version = str_of(ql.get("version"));
    if let Some(md) = ql.get("metadata") {
        m.name = str_of(md.get("name"));
        m.description = str_of(md.get("description"));
        m.homepage = str_of(md.get("contact").and_then(|c| c.get("homepage")));
        if let Some(c) = md.get("contributors").and_then(|x| x.as_object()) {
            for name in c.keys() {
                m.authors.push(name.clone());
            }
        }
    }
    if let Some(deps) = ql.get("depends").and_then(|x| x.as_array()) {
        for d in deps {
            let (id, version, optional) = if let Some(s) = d.as_str() {
                (s.to_string(), None, false)
            } else {
                (
                    str_of(d.get("id")).unwrap_or_default(),
                    str_of(d.get("versions")),
                    d.get("optional")
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false),
                )
            };
            if id.is_empty() {
                continue;
            }
            if id.eq_ignore_ascii_case("minecraft") {
                if m.minecraft.is_none() {
                    m.minecraft = version;
                }
                continue;
            }
            if is_env(&id) {
                continue;
            }
            m.dependencies.push(ModDep {
                id,
                version,
                kind: if optional {
                    "optional".into()
                } else {
                    "required".into()
                },
            });
        }
    }
    Some(m)
}

fn parse_forge_toml(
    s: &str,
    loader: &str,
    manifest_ver: Option<&str>,
) -> Option<ModMeta> {
    let v: toml::Value = toml::from_str(s).ok()?;
    let mods = v.get("mods").and_then(|x| x.as_array())?;
    let first = mods.first()?;
    let mut m = ModMeta {
        loaders: vec![loader.into()],
        ..Default::default()
    };
    m.id = toml_str(first.get("modId"));
    let mut version = toml_str(first.get("version"));
    if version
        .as_deref()
        .map(|s| s.contains("${"))
        .unwrap_or(false)
    {
        version = manifest_ver.map(String::from).or(version);
    }
    m.version = version;
    m.name = toml_str(first.get("displayName"));
    m.description = toml_str(first.get("description"));
    m.homepage = toml_str(first.get("displayURL"));
    if let Some(a) = toml_str(first.get("authors")) {
        m.authors.push(a);
    } else if let Some(arr) = first.get("authors").and_then(|x| x.as_array()) {
        for a in arr {
            if let Some(s) = a.as_str() {
                m.authors.push(s.to_string());
            }
        }
    }
    if let Some(deps_tbl) = v.get("dependencies").and_then(|x| x.as_table()) {
        let our = m.id.clone().unwrap_or_default();
        let lists: Vec<&toml::Value> = match deps_tbl.get(&our) {
            Some(d) => vec![d],
            None => deps_tbl.values().collect(),
        };
        for list in lists {
            let Some(arr) = list.as_array() else { continue };
            for dep in arr {
                let id = toml_str(dep.get("modId")).unwrap_or_default();
                if id.is_empty() {
                    continue;
                }
                let mandatory = dep
                    .get("mandatory")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(true);
                let version = toml_str(dep.get("versionRange"));
                if id.eq_ignore_ascii_case("minecraft") {
                    if m.minecraft.is_none() {
                        m.minecraft = version;
                    }
                    continue;
                }
                if is_env(&id) {
                    continue;
                }
                m.dependencies.push(ModDep {
                    id,
                    version,
                    kind: if mandatory {
                        "required".into()
                    } else {
                        "optional".into()
                    },
                });
            }
        }
    }
    Some(m)
}

fn parse_mcmeta(s: &str) -> Option<ModMeta> {
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    let pack = v.get("pack")?;
    Some(ModMeta {
        loaders: vec!["minecraft".into()],
        pack_format: pack.get("pack_format").and_then(|x| x.as_i64()),
        description: Some(mcmeta_text(pack.get("description")))
            .filter(|s| !s.is_empty()),
        ..Default::default()
    })
}

fn mcmeta_text(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(a)) => a
            .iter()
            .map(|x| mcmeta_text(Some(x)))
            .collect::<Vec<_>>()
            .join(""),
        Some(serde_json::Value::Object(o)) => o
            .get("text")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

fn str_of(v: Option<&serde_json::Value>) -> Option<String> {
    v.and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn toml_str(v: Option<&toml::Value>) -> Option<String> {
    v.and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn cleanup(mut m: ModMeta) -> ModMeta {
    let mut seen = HashSet::new();
    m.dependencies.retain(|d| seen.insert(d.id.to_lowercase()));
    if m.name.as_deref() == Some("") {
        m.name = None;
    }
    if m.description.as_deref() == Some("") {
        m.description = None;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fabric_basic() {
        let s = r#"{"id":"sodium","name":"Sodium","version":"0.5.8","description":"  fast  ","authors":["JellySquid",{"name":"Team"}],"contact":{"homepage":"https://x"},"depends":{"minecraft":"~1.20","fabricloader":">=0.14","fabric-api":"*","cloth-config":">=11"}}"#;
        let m = parse_fabric(s).unwrap();
        assert_eq!(m.id.as_deref(), Some("sodium"));
        assert_eq!(m.name.as_deref(), Some("Sodium"));
        assert_eq!(m.version.as_deref(), Some("0.5.8"));
        assert_eq!(m.description.as_deref(), Some("fast"));
        assert_eq!(m.minecraft.as_deref(), Some("~1.20"));
        assert_eq!(m.authors, vec!["JellySquid", "Team"]);
        let ids: Vec<&str> =
            m.dependencies.iter().map(|d| d.id.as_str()).collect();
        assert!(ids.contains(&"cloth-config"));
        assert!(ids.contains(&"fabric-api"));
        assert!(!ids.contains(&"fabricloader"));
        assert!(!ids.contains(&"minecraft"));
    }

    #[test]
    fn forge_placeholder_version_uses_manifest() {
        let s = r#"
modLoader="javafml"
[[mods]]
modId="create"
version="${file.jarVersion}"
displayName="Create"
[[dependencies.create]]
modId="minecraft"
mandatory=true
versionRange="[1.20,1.21)"
[[dependencies.create]]
modId="flywheel"
mandatory=true
"#;
        let m = parse_forge_toml(s, "forge", Some("0.5.1")).unwrap();
        assert_eq!(m.version.as_deref(), Some("0.5.1"));
        assert_eq!(m.minecraft.as_deref(), Some("[1.20,1.21)"));
        assert_eq!(m.dependencies.len(), 1);
        assert_eq!(m.dependencies[0].id, "flywheel");
    }

    #[test]
    fn mcmeta_resourcepack() {
        let s = r#"{"pack":{"pack_format":15,"description":"My Pack"}}"#;
        let m = parse_mcmeta(s).unwrap();
        assert_eq!(m.pack_format, Some(15));
        assert_eq!(m.description.as_deref(), Some("My Pack"));
    }
}
