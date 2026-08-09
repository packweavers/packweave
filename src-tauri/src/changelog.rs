use crate::packdiff::{ItemChange, PackDiff};
use crate::providers::ProviderId;

fn loader_label(l: &str) -> String {
    if l.is_empty() {
        return String::new();
    }
    if l == "vanilla" {
        return "Vanilla".into();
    }
    let mut c = l.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn loader_full(loader: &str, lv: &Option<String>) -> String {
    match lv {
        Some(v) if !v.is_empty() => format!("{} {v}", loader_label(loader)),
        _ => loader_label(loader),
    }
}

fn ver(v: &Option<String>) -> String {
    match v {
        Some(s) if !s.is_empty() => format!(" {s}"),
        _ => String::new(),
    }
}

fn push_section(out: &mut String, title: &str, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    out.push_str("### ");
    out.push_str(title);
    out.push('\n');
    for l in lines {
        out.push_str(l);
        out.push('\n');
    }
    out.push('\n');
}

fn name_md(
    c: &ItemChange,
    links: bool,
    provider: Option<ProviderId>,
) -> String {
    if links && provider == Some(ProviderId::Modrinth) && !c.slug.is_empty() {
        format!("[{}](https://modrinth.com/project/{})", c.name, c.slug)
    } else {
        c.name.clone()
    }
}

pub fn render(
    diff: &PackDiff,
    heading: Option<&str>,
    links: bool,
    for_file: bool,
) -> String {
    let mut s = String::new();
    if let Some(h) = heading {
        let h = h.trim();
        if !h.is_empty() {
            s.push_str("## ");
            s.push_str(h);
            s.push_str("\n\n");
        }
    }

    let mut added = Vec::new();
    let mut updated = Vec::new();
    let mut removed = Vec::new();
    let mut disabled = Vec::new();
    let mut enabled = Vec::new();
    let mut changed = Vec::new();

    if let Some(env) = &diff.env {
        if env.from_minecraft != env.to_minecraft {
            changed.push(format!(
                "- Minecraft {} → {}",
                env.from_minecraft, env.to_minecraft
            ));
        }
        if env.from_loader != env.to_loader
            || env.from_loader_version != env.to_loader_version
        {
            changed.push(format!(
                "- Loader {} → {}",
                loader_full(&env.from_loader, &env.from_loader_version),
                loader_full(&env.to_loader, &env.to_loader_version)
            ));
        }
    }

    for c in &diff.items {
        if for_file && c.disabled && c.kind != "disabled" {
            continue;
        }
        match c.kind.as_str() {
            "added" => added.push(format!(
                "- {}{}",
                name_md(c, links, c.to_provider),
                ver(&c.to_version)
            )),
            "removed" => removed.push(format!(
                "- {}{}",
                name_md(c, links, c.from_provider),
                ver(&c.from_version)
            )),
            "disabled" if for_file => removed.push(format!(
                "- {}{}",
                name_md(c, links, c.to_provider),
                ver(&c.to_version)
            )),
            "enabled" if for_file => added.push(format!(
                "- {}{}",
                name_md(c, links, c.to_provider),
                ver(&c.to_version)
            )),
            "disabled" => {
                disabled.push(format!("- {}", name_md(c, links, c.to_provider)))
            }
            "enabled" => {
                enabled.push(format!("- {}", name_md(c, links, c.to_provider)))
            }
            "updated" => updated.push(format!(
                "- {} {} → {}",
                name_md(c, links, c.to_provider),
                c.from_version.as_deref().unwrap_or("?"),
                c.to_version.as_deref().unwrap_or("?")
            )),
            "provider" => changed.push(format!(
                "- {}: source {} → {}",
                name_md(c, links, c.to_provider),
                c.from_provider.map(|p| p.as_str()).unwrap_or("?"),
                c.to_provider.map(|p| p.as_str()).unwrap_or("?")
            )),
            "resided" => changed.push(format!(
                "- {}: side {} → {}",
                name_md(c, links, c.to_provider),
                c.from_side.as_deref().unwrap_or("?"),
                c.to_side.as_deref().unwrap_or("?")
            )),
            _ => {}
        }
    }

    for f in &diff.files {
        changed.push(format!("- {} {}", f.status, f.path));
    }

    push_section(&mut s, "Added", &added);
    push_section(&mut s, "Updated", &updated);
    push_section(&mut s, "Removed", &removed);
    push_section(&mut s, "Enabled", &enabled);
    push_section(&mut s, "Disabled", &disabled);
    push_section(&mut s, "Changed", &changed);

    if added.is_empty()
        && updated.is_empty()
        && removed.is_empty()
        && enabled.is_empty()
        && disabled.is_empty()
        && changed.is_empty()
    {
        s.push_str("_No pack changes._\n");
    }

    format!("{}\n", s.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packdiff::{EnvDiff, FileChange, ItemChange, PackDiff};

    fn item(
        kind: &str,
        name: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> ItemChange {
        ItemChange {
            kind: kind.into(),
            name: name.into(),
            project_type: crate::ptype::ProjectType::Mod,
            slug: name.to_lowercase(),
            from_version: from.map(Into::into),
            to_version: to.map(Into::into),
            from_side: None,
            to_side: None,
            from_provider: None,
            to_provider: None,
            disabled: false,
        }
    }

    #[test]
    fn groups_into_sections() {
        let diff = PackDiff {
            items: vec![
                item("added", "Sodium", None, Some("0.5")),
                item("updated", "Create", Some("0.5"), Some("0.6")),
                item("removed", "OptiFine", Some("1.0"), None),
            ],
            env: None,
            files: vec![FileChange {
                status: "modified".into(),
                path: "config/foo.json".into(),
            }],
        };
        let md = render(&diff, Some("v1.2.0"), false, false);
        assert!(md.starts_with("## v1.2.0"));
        assert!(md.contains("### Added\n- Sodium 0.5"));
        assert!(md.contains("### Updated\n- Create 0.5 → 0.6"));
        assert!(md.contains("### Removed\n- OptiFine 1.0"));
        assert!(md.contains("- modified config/foo.json"));
    }

    #[test]
    fn env_change_lands_in_changed() {
        let diff = PackDiff {
            items: vec![],
            files: vec![],
            env: Some(EnvDiff {
                from_minecraft: "1.20.1".into(),
                to_minecraft: "1.21".into(),
                from_loader: "fabric".into(),
                to_loader: "fabric".into(),
                from_loader_version: None,
                to_loader_version: None,
            }),
        };
        let md = render(&diff, None, false, false);
        assert!(md.contains("### Changed"));
        assert!(md.contains("Minecraft 1.20.1 → 1.21"));
    }

    #[test]
    fn empty_diff_says_no_changes() {
        let diff = PackDiff {
            items: vec![],
            env: None,
            files: vec![],
        };
        assert!(render(&diff, None, false, false).contains("No pack changes"));
    }

    #[test]
    fn links_modrinth_mods_when_enabled() {
        let mut added = item("added", "Sodium", None, Some("0.5"));
        added.slug = "sodium".into();
        added.to_provider = Some(ProviderId::Modrinth);
        let mut cf = item("added", "JEI", None, Some("1.0"));
        cf.slug = "jei".into();
        cf.to_provider = Some(ProviderId::Curseforge);
        let diff = PackDiff {
            items: vec![added, cf],
            env: None,
            files: vec![],
        };
        let linked = render(&diff, None, true, false);
        assert!(linked
            .contains("- [Sodium](https://modrinth.com/project/sodium) 0.5"));
        assert!(linked.contains("- JEI 1.0"));
        let plain = render(&diff, None, false, false);
        assert!(plain.contains("- Sodium 0.5"));
        assert!(!plain.contains("modrinth.com"));
    }

    #[test]
    fn disable_enable_render_by_audience() {
        let mut dis = item("disabled", "Sodium", Some("0.5"), Some("0.5"));
        dis.disabled = true;
        let mut en = item("enabled", "Lithium", Some("0.11"), Some("0.11"));
        en.disabled = false;
        let diff = PackDiff {
            items: vec![dis, en],
            env: None,
            files: vec![],
        };

        let interactive = render(&diff, None, false, false);
        assert!(interactive.contains("### Disabled\n- Sodium"));
        assert!(interactive.contains("### Enabled\n- Lithium"));
        assert!(!interactive.contains("### Removed"));

        let file = render(&diff, None, false, true);
        assert!(file.contains("### Removed\n- Sodium"));
        assert!(file.contains("### Added\n- Lithium"));
        assert!(!file.contains("Disabled"));
    }

    #[test]
    fn file_mode_omits_changes_to_still_disabled_content() {
        let mut upd = item("updated", "Sodium", Some("0.5"), Some("0.6"));
        upd.disabled = true;
        let diff = PackDiff {
            items: vec![upd],
            env: None,
            files: vec![],
        };
        assert!(render(&diff, None, false, true).contains("No pack changes"));
        assert!(render(&diff, None, false, false).contains("### Updated"));
    }
}
