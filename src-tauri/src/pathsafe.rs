use std::path::PathBuf;

#[cfg(windows)]
fn component_for_fs(part: &str) -> String {
    let mut s: String = part
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') || (c as u32) < 32 {
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

pub fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains(':')
}
