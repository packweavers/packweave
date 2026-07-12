use crate::providers::ProviderId;
use crate::ptype::ProjectType;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ParsedRef {
    pub provider: Option<ProviderId>,
    pub reference: String,
    pub project_type: Option<ProjectType>,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCandidate {
    pub provider: ProviderId,
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub project_type: ProjectType,
    pub icon_url: Option<String>,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkFailure {
    pub raw: String,
    pub reason: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkLookup {
    pub found: Vec<BulkCandidate>,
    pub failed: Vec<BulkFailure>,
}

fn modrinth_type(seg: &str) -> Option<ProjectType> {
    match seg {
        "mod" | "plugin" | "datapack" => Some(ProjectType::Mod),
        "resourcepack" => Some(ProjectType::Resourcepack),
        "shader" => Some(ProjectType::Shader),
        _ => None,
    }
}

fn curseforge_type(seg: &str) -> Option<ProjectType> {
    match seg {
        "mc-mods" | "data-packs" => Some(ProjectType::Mod),
        "texture-packs" => Some(ProjectType::Resourcepack),
        "shaders" => Some(ProjectType::Shader),
        _ => None,
    }
}

fn clean_slug(s: &str) -> String {
    s.split(['?', '#'])
        .next()
        .unwrap_or(s)
        .trim_matches('/')
        .to_string()
}

fn is_slug(s: &str) -> bool {
    s.len() >= 2
        && s.chars().any(|c| c.is_ascii_alphabetic())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn parse_url(token: &str) -> Option<ParsedRef> {
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
        return Some(ParsedRef {
            provider: Some(ProviderId::Modrinth),
            reference: slug,
            project_type: Some(ptype),
            raw: token.to_string(),
        });
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
        return Some(ParsedRef {
            provider: Some(ProviderId::Curseforge),
            reference: slug,
            project_type: Some(ptype),
            raw: token.to_string(),
        });
    }
    None
}

pub fn parse_refs(text: &str) -> Vec<ParsedRef> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let mut had_url = false;
        for tok in &tokens {
            if let Some(r) = parse_url(tok) {
                had_url = true;
                let key =
                    format!("{:?}:{}", r.provider, r.reference.to_lowercase());
                if seen.insert(key) {
                    out.push(r);
                }
            }
        }
        if !had_url && tokens.len() == 1 && is_slug(tokens[0]) {
            let key = format!("None:{}", tokens[0].to_lowercase());
            if seen.insert(key) {
                out.push(ParsedRef {
                    provider: None,
                    reference: tokens[0].to_string(),
                    project_type: None,
                    raw: tokens[0].to_string(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modrinth_and_curseforge_urls_in_prose() {
        let text = "try https://modrinth.com/mod/sodium and \
                    www.curseforge.com/minecraft/mc-mods/jei plus a shader \
                    https://modrinth.com/shader/complementary-reimagined";
        let refs = parse_refs(text);
        assert_eq!(refs.len(), 3);
        assert_eq!(refs[0].provider, Some(ProviderId::Modrinth));
        assert_eq!(refs[0].reference, "sodium");
        assert_eq!(refs[0].project_type, Some(ProjectType::Mod));
        assert_eq!(refs[1].provider, Some(ProviderId::Curseforge));
        assert_eq!(refs[1].reference, "jei");
        assert_eq!(refs[2].project_type, Some(ProjectType::Shader));
    }

    #[test]
    fn bare_slug_lines_only_no_prose() {
        let refs =
            parse_refs("sodium\nlithium\nadd these two please\nfabric-api");
        let slugs: Vec<&str> =
            refs.iter().map(|r| r.reference.as_str()).collect();
        assert_eq!(slugs, vec!["sodium", "lithium", "fabric-api"]);
        assert!(refs.iter().all(|r| r.provider.is_none()));
    }

    #[test]
    fn dedupes_and_strips_query() {
        let refs = parse_refs(
            "https://modrinth.com/mod/sodium?foo=1\nhttps://modrinth.com/mod/sodium/versions",
        );
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].reference, "sodium");
    }
}
