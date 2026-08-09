use std::path::PathBuf;

#[cfg(windows)]
fn component_for_fs(part: &str) -> String {
    let mut s: String = part
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')
                || (c as u32) < 32
            {
                '_'
            } else {
                c
            }
        })
        .collect();
    while s.ends_with('.') || s.ends_with(' ') {
        s.pop();
    }
    if s.is_empty() {
        s.push('_');
    }
    let stem = s.split('.').next().unwrap_or_default().to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem[3..].chars().all(|c| c.is_ascii_digit()));
    if reserved {
        s.insert(0, '_');
    }
    s
}

#[cfg(not(windows))]
fn component_for_fs(part: &str) -> String {
    part.to_string()
}

pub fn safe_zip_rel(rel: &str) -> Option<PathBuf> {
    let norm = rel.replace('\\', "/");
    let mut out = PathBuf::new();
    for raw in norm.split('/') {
        let part = raw.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return None;
        }
        out.push(component_for_fs(part));
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn safe_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    if base.is_empty() || base == "." || base == ".." {
        return "file".into();
    }
    component_for_fs(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal() {
        assert!(safe_zip_rel("../evil.jar").is_none());
        assert!(safe_zip_rel("mods/../../evil.jar").is_none());
        assert!(safe_zip_rel("").is_none());
    }

    #[test]
    fn normalizes_backslashes() {
        assert_eq!(
            safe_zip_rel("mods\\sodium.jar").unwrap(),
            PathBuf::from("mods/sodium.jar")
        );
    }

    #[test]
    fn strips_leading_roots() {
        assert_eq!(
            safe_zip_rel("/mods/sodium.jar").unwrap(),
            PathBuf::from("mods/sodium.jar")
        );
    }

    #[test]
    fn filename_takes_basename() {
        assert_eq!(safe_filename("a/b/../c.jar"), "c.jar");
        assert_eq!(safe_filename(".."), "file");
    }
}
