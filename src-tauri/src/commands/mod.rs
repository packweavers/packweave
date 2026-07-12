use serde::Serialize;
use std::path::Path;

use crate::content;
use crate::curseforge::CurseForge;
use crate::lockfile::{self, Lockfile};
use crate::manifest::{self, Manifest};
use crate::modrinth::Modrinth;
use crate::resolver;
use crate::validate::{self, Health, Validation};

mod catalog;
mod content_ops;
mod exports;
mod files;
mod gitops;
mod instances;
mod pack;
mod settings;

pub use catalog::*;
pub use content_ops::*;
pub use exports::*;
pub use files::*;
pub use gitops::*;
pub use instances::*;
pub use pack::*;
pub use settings::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackState {
    pub dir: String,
    pub manifest: Manifest,
    pub lockfile: Option<Lockfile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackResolved {
    pub manifest: Manifest,
    pub lockfile: Lockfile,
    pub validations: Vec<Validation>,
    pub health: Health,
}

pub(crate) fn es<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub(crate) async fn resolve_loader_version(
    mr: &Modrinth,
    manifest: &Manifest,
) -> Option<String> {
    match manifest.loader_version.as_deref() {
        Some(v) if !v.is_empty() => Some(v.to_string()),
        _ if manifest.loader != "vanilla" => mr
            .loader_versions(&manifest.loader, &manifest.minecraft)
            .await
            .ok()
            .and_then(|v| v.into_iter().next()),
        _ => None,
    }
}

pub(crate) async fn do_resolve(
    mr: &Modrinth,
    cf: &CurseForge,
    dir: &Path,
) -> anyhow::Result<PackResolved> {
    let manifest = manifest::read(dir)?;
    let authored = content::authored(dir);
    let out = resolver::resolve(mr, cf, &manifest, &authored).await?;
    let mut combined = out.locked.clone();
    let resolved: std::collections::HashSet<String> =
        out.locked.iter().map(|i| i.project_id()).collect();
    combined.extend(
        content::read_all(dir)
            .into_iter()
            .filter(|i| i.disabled && !resolved.contains(&i.project_id())),
    );
    content::write_resolved(dir, &manifest, &combined)?;
    let lockfile = Lockfile {
        minecraft: manifest.minecraft.clone(),
        loader: manifest.loader.clone(),
        loader_version: resolve_loader_version(mr, &manifest).await,
        mods: combined.iter().map(content::to_locked).collect(),
    };
    lockfile::cache_put(dir, &lockfile);
    let (validations, health) = validate::build(&out, &lockfile);
    Ok(PackResolved {
        manifest,
        lockfile,
        validations,
        health,
    })
}
