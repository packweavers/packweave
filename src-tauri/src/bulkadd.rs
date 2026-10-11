use crate::modrinth::SearchHit;
use crate::providers::ProviderId;
use crate::ptype::ProjectType;
use serde::Serialize;
use ts_rs::TS;

pub const EXACT: u8 = 3;

#[derive(Debug, Clone)]
pub struct ParsedRef {
    pub reference: String,
    pub project_type: Option<ProjectType>,
    pub raw: String,
    pub line: usize,
    pub by_name: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct BulkCandidate {
    pub provider: ProviderId,
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub project_type: ProjectType,
    pub icon_url: Option<String>,
    pub raw: String,
    pub line: usize,
    pub guess: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct BulkFailure {
    pub raw: String,
    pub reason: String,
    pub line: usize,
}

#[derive(Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct BulkLookup {
    pub found: Vec<BulkCandidate>,
    pub in_pack: Vec<BulkCandidate>,
    pub failed: Vec<BulkFailure>,
}

fn modrinth_type(seg: &str) -> Option<ProjectType> {
    match seg {
        "mod" | "plugin" => Some(ProjectType::Mod),
        "datapack" => Some(ProjectType::Datapack),
        "resourcepack" => Some(ProjectType::Resourcepack),
        "shader" => Some(ProjectType::Shader),
        _ => None,
    }
}

fn curseforge_type(seg: &str) -> Option<ProjectType> {
    match seg {
        "mc-mods" => Some(ProjectType::Mod),
        "data-packs" => Some(ProjectType::Datapack),
        "texture-packs" => Some(ProjectType::Resourcepack),
        "shaders" => Some(ProjectType::Shader),
        _ => None,
    }
}

fn clean_slug(s: &str) -> String {
    s.split(['?', '#'])
        .next()
        .unwrap_or(s)
        .trim_matches(|c: char| matches!(c, '/' | ')' | ']' | '>' | ',' | '.' | '"' | '\''))
        .to_string()
}

fn strip_bullet(line: &str) -> &str {
    let text = line
        .trim()
        .trim_start_matches(['-', '*', '+', '>', '\u{2022}'])
        .trim_start();
    let digits = text.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        let rest = &text[digits..];
        if let Some(after) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')) {
            return after.trim_start();
        }
    }
    text
}

fn jar_slug(token: &str) -> Option<String> {
    let lower = token.to_lowercase();
    let stem = lower
        .strip_suffix(".jar")
        .or_else(|| lower.strip_suffix(".zip"))?;
    let name = stem.rsplit(['/', '\\']).next()?;
    let mut parts = Vec::new();
    for part in name.split(['-', '_', '+', ' ']) {
        let versioned = part.starts_with(|c: char| c.is_ascii_digit())
            || part.starts_with("mc1")
            || (part.starts_with('v') && part[1..].starts_with(|c: char| c.is_ascii_digit()));
        let loader = matches!(
            part,
            "fabric" | "forge" | "neoforge" | "quilt" | "mc" | "universal" | "all"
        );
        if part.is_empty() || versioned || loader {
            break;
        }
        parts.push(part);
    }
    (!parts.is_empty()).then(|| parts.join("-"))
}

fn is_slug(s: &str) -> bool {
    s.len() >= 2
        && s.chars().any(|c| c.is_ascii_alphabetic())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn parse_url(token: &str) -> Option<(String, ProjectType)> {
    let lower = token.to_lowercase();
    if let Some(idx) = lower.find("modrinth.com/") {
        let rest = &token[idx + "modrinth.com/".len()..];
        let mut segs = rest.split('/').filter(|s| !s.is_empty());
        let kind = segs.next()?;
        let slug = clean_slug(segs.next()?);
        let ptype = modrinth_type(kind)?;
        if slug.is_empty() {
            return None;
        }
        return Some((slug, ptype));
    }
    if let Some(idx) = lower.find("curseforge.com/minecraft/") {
        let rest = &token[idx + "curseforge.com/minecraft/".len()..];
        let mut segs = rest.split('/').filter(|s| !s.is_empty());
        let cat = segs.next()?;
        let slug = clean_slug(segs.next()?);
        let ptype = curseforge_type(cat)?;
        if slug.is_empty() {
            return None;
        }
        return Some((slug, ptype));
    }
    None
}

pub fn normal(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

pub fn closeness(wanted: &str, hit: &SearchHit) -> u8 {
    if wanted.is_empty() {
        return 0;
    }
    let title = normal(&hit.title);
    let slug = normal(&hit.slug);
    if title == wanted || slug == wanted {
        EXACT
    } else if title.starts_with(wanted) || slug.starts_with(wanted) {
        2
    } else if title.contains(wanted) || slug.contains(wanted) {
        1
    } else {
        0
    }
}

pub fn has_exact(wanted: &str, hits: &[SearchHit]) -> bool {
    hits.iter().any(|hit| closeness(wanted, hit) == EXACT)
}

pub fn best<'a>(
    wanted: &str,
    found: &'a [(ProjectType, Vec<SearchHit>)],
) -> Option<(ProjectType, &'a SearchHit, u8)> {
    let mut best: Option<(ProjectType, &SearchHit, u8)> = None;
    for (kind, hits) in found {
        for hit in hits {
            let score = closeness(wanted, hit);
            if best.is_none_or(|(_, _, top)| score > top) {
                best = Some((*kind, hit, score));
            }
        }
    }
    best
}

pub fn parse_refs(text: &str) -> Vec<ParsedRef> {
    let mut out: Vec<ParsedRef> = Vec::new();
    for (line, raw_line) in text.lines().enumerate() {
        let body = strip_bullet(raw_line);
        if body.is_empty() || body.starts_with('#') || body.starts_with("//") {
            continue;
        }
        let mut push =
            |reference: String, project_type: Option<ProjectType>, raw: &str, by_name: bool| {
                out.push(ParsedRef {
                    reference,
                    project_type,
                    raw: raw.to_string(),
                    line,
                    by_name,
                });
            };
        for piece in body
            .split([',', ';', '|', '\t'])
            .map(|p| strip_bullet(p.trim()))
            .filter(|p| !p.is_empty())
        {
            let urls: Vec<(String, ProjectType, &str)> = piece
                .split_whitespace()
                .filter_map(|token| parse_url(token).map(|(slug, kind)| (slug, kind, token)))
                .collect();
            if !urls.is_empty() {
                for (slug, kind, raw) in urls {
                    push(slug, Some(kind), raw, false);
                }
            } else if let Some(slug) = jar_slug(piece) {
                push(slug, None, piece, false);
            } else if is_slug(piece) {
                push(piece.to_string(), None, piece, false);
            } else {
                push(piece.to_string(), None, piece, true);
            }
        }
    }
    out
}
