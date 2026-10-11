use std::collections::HashSet;
use std::path::Path;

use crate::content;
use crate::instance;
use crate::modmeta;
use crate::packlocal;

const ROOTS: [&str; 2] = ["config", "defaultconfigs"];
const SUFFIXES: [&str; 8] = [
    "common", "client", "server", "config", "configs", "settings", "options", "general",
];
const LIMIT: usize = 40;

fn normalize(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn stem(name: &str) -> String {
    let base = normalize(name.split('.').next().unwrap_or(name));
    for suffix in SUFFIXES {
        if let Some(rest) = base.strip_suffix(suffix) {
            if rest.len() >= 3 {
                return rest.to_string();
            }
        }
    }
    base
}

fn jar_ids(jar: &Path, keys: &mut HashSet<String>) -> bool {
    let Some(meta) = modmeta::read_archive(jar) else {
        return false;
    };
    keys.extend(meta.ids.iter().map(|id| normalize(id)));
    true
}

fn keys_for(pack_dir: &Path, key: &str) -> HashSet<String> {
    let mut keys = HashSet::new();
    if let Some(rel) = key.strip_prefix("local:") {
        let jar = instance::overrides_dir(pack_dir).join(rel);
        jar_ids(&jar, &mut keys);
        return keys;
    }
    let Some(item) = content::read_all(pack_dir)
        .into_iter()
        .find(|i| content::matches(i, key))
    else {
        return keys;
    };
    keys.insert(normalize(&item.name));
    let Some(source) = item.active() else {
        return keys;
    };
    keys.insert(normalize(&source.slug));
    if let Some(game_dir) = packlocal::read(pack_dir).instance_dir {
        let folder = Path::new(&game_dir)
            .join(crate::ptype::Folders::load(pack_dir).install_dir(item.project_type));
        let disabled = format!("{}.disabled", source.filename);
        for file in [source.filename.as_str(), disabled.as_str()] {
            if !file.is_empty() && jar_ids(&folder.join(file), &mut keys) {
                break;
            }
        }
    }
    keys
}

fn collect(base: &Path, path: &Path, out: &mut Vec<String>) {
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                collect(base, &entry.path(), out);
            }
        }
    } else if let Ok(rel) = path.strip_prefix(base) {
        out.push(format!(
            "{}/{}",
            instance::OVERRIDES_DIR,
            rel.to_string_lossy().replace('\\', "/")
        ));
    }
}

pub fn for_content(pack_dir: &Path, key: &str) -> Vec<String> {
    let keys: HashSet<String> = keys_for(pack_dir, key)
        .into_iter()
        .filter(|k| k.len() >= 3)
        .collect();
    if keys.is_empty() {
        return Vec::new();
    }
    let overrides = instance::overrides_dir(pack_dir);
    let mut out = Vec::new();
    for root in ROOTS {
        let Ok(entries) = std::fs::read_dir(overrides.join(root)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if keys.contains(&stem(&name)) {
                collect(&overrides, &entry.path(), &mut out);
            }
        }
    }
    out.sort();
    out.truncate(LIMIT);
    out
}
