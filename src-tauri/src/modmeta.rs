use serde::Serialize;
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts", rename = "ArchiveDep")]
pub struct ModDep {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub version: Option<String>,
    #[ts(type = "'required' | 'optional' | 'incompatible' | 'discouraged'")]
    pub kind: String,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts", rename = "ArchiveMeta")]
pub struct ModMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "'client' | 'server'", optional)]
    pub side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub icon: Option<String>,
    #[serde(skip)]
    icon_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<ModDep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub minecraft: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
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
    let meta = parse_archive(&mut zip)?;
    Some(finish(&mut zip, meta))
}

fn parse_archive(zip: &mut zip::ZipArchive<std::fs::File>) -> Option<ModMeta> {
    let names: HashSet<String> = zip.file_names().map(String::from).collect();

    if names.contains("fabric.mod.json") {
        if let Some(m) = entry(zip, "fabric.mod.json").and_then(|s| parse_fabric(&s)) {
            return Some(m);
        }
    }
    if names.contains("quilt.mod.json") {
        if let Some(m) = entry(zip, "quilt.mod.json").and_then(|s| parse_quilt(&s)) {
            return Some(m);
        }
    }
    let manifest_ver = if names.contains("META-INF/MANIFEST.MF") {
        entry(zip, "META-INF/MANIFEST.MF").and_then(|s| manifest_version(&s))
    } else {
        None
    };
    if names.contains("META-INF/neoforge.mods.toml") {
        if let Some(m) = entry(zip, "META-INF/neoforge.mods.toml")
            .and_then(|s| parse_forge_toml(&s, "neoforge", manifest_ver.as_deref()))
        {
            return Some(m);
        }
    }
    if names.contains("META-INF/mods.toml") {
        if let Some(m) = entry(zip, "META-INF/mods.toml")
            .and_then(|s| parse_forge_toml(&s, "forge", manifest_ver.as_deref()))
        {
            return Some(m);
        }
    }
    if names.contains("pack.mcmeta") {
        if let Some(mut m) = entry(zip, "pack.mcmeta").and_then(|s| parse_mcmeta(&s)) {
            let data = names.iter().any(|n| n.starts_with("data/"));
            let assets = names.iter().any(|n| n.starts_with("assets/"));
            if data && !assets {
                m.loaders = vec!["datapack".into()];
            }
            return Some(m);
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

fn finish(zip: &mut zip::ZipArchive<std::fs::File>, mut m: ModMeta) -> ModMeta {
    if let Some(id) = m.id.clone() {
        if !m.ids.contains(&id) {
            m.ids.insert(0, id);
        }
    }
    m.icon = m
        .icon_path
        .take()
        .map(|p| p.trim_start_matches('/').to_string())
        .and_then(|p| icon_data(zip, &p));
    cleanup(m)
}

fn icon_data(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    use base64::Engine;
    const MAX_ICON: u64 = 256 * 1024;
    let mut f = zip.by_name(name).ok()?;
    if f.size() > MAX_ICON {
        return None;
    }
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).ok()?;
    let mime = if name.to_lowercase().ends_with(".jpg") || name.to_lowercase().ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    };
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn icon_of(v: Option<&serde_json::Value>) -> Option<String> {
    match v? {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(sizes) => sizes
            .iter()
            .max_by_key(|(size, _)| size.parse::<u32>().unwrap_or(0))
            .and_then(|(_, path)| path.as_str().map(String::from)),
        _ => None,
    }
}

fn provided(v: Option<&serde_json::Value>) -> Vec<String> {
    v.and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|p| p.as_str().or_else(|| p.get("id").and_then(|i| i.as_str())))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
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
    m.ids = provided(v.get("provides"));
    m.name = str_of(v.get("name"));
    m.version = str_of(v.get("version"));
    m.description = str_of(v.get("description"));
    m.homepage = str_of(v.get("contact").and_then(|c| c.get("homepage")));
    m.icon_path = icon_of(v.get("icon"));
    m.side = match v.get("environment").and_then(|e| e.as_str()) {
        Some("client") => Some("client".into()),
        Some("server") => Some("server".into()),
        _ => None,
    };
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

fn fabric_deps(v: &serde_json::Value, field: &str, kind: &str, m: &mut ModMeta) {
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
    m.ids = provided(ql.get("provides"));
    m.version = str_of(ql.get("version"));
    m.side = match v
        .get("minecraft")
        .and_then(|mc| mc.get("environment"))
        .and_then(|e| e.as_str())
    {
        Some("client") => Some("client".into()),
        Some("dedicated_server") => Some("server".into()),
        _ => None,
    };
    if let Some(md) = ql.get("metadata") {
        m.name = str_of(md.get("name"));
        m.description = str_of(md.get("description"));
        m.icon_path = icon_of(md.get("icon"));
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
                    d.get("optional").and_then(|x| x.as_bool()).unwrap_or(false),
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

fn parse_forge_toml(s: &str, loader: &str, manifest_ver: Option<&str>) -> Option<ModMeta> {
    let v: toml::Value = toml::from_str(s).ok()?;
    let mods = v.get("mods").and_then(|x| x.as_array())?;
    let first = mods.first()?;
    let mut m = ModMeta {
        loaders: vec![loader.into()],
        ..Default::default()
    };
    m.id = toml_str(first.get("modId"));
    m.ids = mods
        .iter()
        .filter_map(|entry| toml_str(entry.get("modId")))
        .collect();
    m.icon_path = toml_str(first.get("logoFile")).or_else(|| toml_str(v.get("logoFile")));
    if toml_str(first.get("displayTest")).as_deref() == Some("IGNORE_SERVER_VERSION") {
        m.side = Some("client".into());
    }
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
                let kind = match toml_str(dep.get("type"))
                    .map(|t| t.to_lowercase())
                    .as_deref()
                {
                    Some("required") => "required",
                    Some("optional") => "optional",
                    Some("incompatible") => "incompatible",
                    Some("discouraged") => "discouraged",
                    _ if mandatory => "required",
                    _ => "optional",
                };
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
                    kind: kind.into(),
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
        icon_path: Some("pack.png".into()),
        pack_format: pack.get("pack_format").and_then(|x| x.as_i64()),
        description: Some(mcmeta_text(pack.get("description"))).filter(|s| !s.is_empty()),
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
