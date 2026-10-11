use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;
use ts_rs::TS;

use crate::cfpack;
use crate::curseforge::CurseForge;
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::mrpack;
use crate::packignore::PackFile;
use crate::ptype::Folders;
use crate::USER_AGENT;

#[derive(Serialize, Default, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct Bundling {
    pub bundled: Vec<String>,
    pub left_out: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn publish_modrinth(
    lock: &Lockfile,
    manifest: &Manifest,
    overrides: &[PackFile],
    project_id: &str,
    version_number: &str,
    changelog: &str,
    env: &str,
    token: &str,
) -> Result<String> {
    if token.is_empty() {
        bail!("Add your Modrinth token in Settings first.");
    }
    if project_id.is_empty() {
        bail!("Enter the Modrinth project id.");
    }
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
    let project_id = modrinth_project(&client, token, project_id.trim()).await?;
    let tmp = std::env::temp_dir().join(format!("packweave-{}.mrpack", file_safe(version_number)));
    mrpack::export(
        lock,
        &manifest.name,
        version_number,
        &tmp,
        env,
        overrides,
        &Folders::of(manifest),
    )
    .await?;
    let bytes = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);

    let loader = if manifest.loader == "vanilla" {
        "minecraft"
    } else {
        manifest.loader.as_str()
    };
    let data = json!({
        "name": version_number,
        "version_number": version_number,
        "changelog": changelog,
        "dependencies": [],
        "game_versions": [manifest.minecraft],
        "version_type": manifest.channel.as_str(),
        "loaders": [loader],
        "featured": true,
        "status": "listed",
        "project_id": project_id,
        "file_parts": ["file"],
        "primary_file": "file",
    });

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(format!(
            "{}-{}.mrpack",
            file_safe(&manifest.name),
            file_safe(version_number)
        ))
        .mime_str("application/x-modrinth-modpack+zip")?;
    let form = reqwest::multipart::Form::new()
        .text("data", serde_json::to_string(&data)?)
        .part("file", part);

    let resp = client
        .post("https://api.modrinth.com/v2/version")
        .header("Authorization", token)
        .multipart(form)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!(
            "Modrinth rejected the upload ({status}). {}",
            truncate(&text)
        );
    }
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({}));
    let id = v["id"].as_str().unwrap_or_default();
    Ok(format!(
        "https://modrinth.com/project/{project_id}/version/{id}"
    ))
}

#[allow(clippy::too_many_arguments)]
pub async fn publish_curseforge(
    cf: &CurseForge,
    lock: &Lockfile,
    manifest: &Manifest,
    overrides: &[PackFile],
    project_id: &str,
    version_number: &str,
    changelog: &str,
    env: &str,
    token: &str,
) -> Result<String> {
    if token.is_empty() {
        bail!("Add your CurseForge upload token in Settings first.");
    }
    let project_id = project_id.trim();
    if project_id.is_empty() {
        bail!("Enter the CurseForge project id.");
    }
    if !project_id.chars().all(|c| c.is_ascii_digit()) {
        bail!("Enter the CurseForge project ID, the number on the project's page, not its name.");
    }
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
    let game_versions =
        cf_game_version_ids(&client, token, &manifest.minecraft, &manifest.loader).await?;
    let tmp = std::env::temp_dir().join(format!("packweave-{}-cf.zip", file_safe(version_number)));
    cfpack::export(
        cf,
        lock,
        &manifest.name,
        version_number,
        &tmp,
        env,
        overrides,
        &Folders::of(manifest),
    )
    .await?;
    let bytes = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);

    let metadata = json!({
        "changelog": changelog,
        "changelogType": "markdown",
        "displayName": format!("{} {}", manifest.name, version_number),
        "releaseType": manifest.channel.as_str(),
        "gameVersions": game_versions,
    });

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(format!(
            "{}-{}.zip",
            file_safe(&manifest.name),
            file_safe(version_number)
        ))
        .mime_str("application/zip")?;
    let form = reqwest::multipart::Form::new()
        .text("metadata", serde_json::to_string(&metadata)?)
        .part("file", part);

    let url = format!("https://minecraft.curseforge.com/api/projects/{project_id}/upload-file");
    let resp = client
        .post(&url)
        .header("X-Api-Token", token)
        .multipart(form)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!(
            "CurseForge rejected the upload ({status}). {}",
            truncate(&text)
        );
    }
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({}));
    let id = v["id"].as_i64().map(|n| n.to_string()).unwrap_or_default();
    Ok(format!(
        "Uploaded to CurseForge project {project_id} (file {id})."
    ))
}

async fn modrinth_project(client: &reqwest::Client, token: &str, project: &str) -> Result<String> {
    let resp = client
        .get(format!("https://api.modrinth.com/v2/project/{project}"))
        .header("Authorization", token)
        .send()
        .await?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(project.to_string());
    }
    if !status.is_success() {
        bail!("Couldn't look up {project} on Modrinth ({status}).");
    }
    let found: serde_json::Value = resp.json().await?;
    if let Some(kind) = found["project_type"].as_str().filter(|k| *k != "modpack") {
        bail!("{project} on Modrinth is a {kind}, not a modpack.");
    }
    Ok(found["id"].as_str().unwrap_or(project).to_string())
}

async fn cf_list(
    client: &reqwest::Client,
    token: &str,
    url: &str,
) -> Result<Vec<serde_json::Value>> {
    let resp = client.get(url).header("X-Api-Token", token).send().await?;
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        bail!("CurseForge didn't accept your upload token. Check it in Settings.");
    }
    if !status.is_success() {
        bail!("Couldn't load CurseForge's game versions ({status}).");
    }
    Ok(resp.json().await?)
}

async fn cf_game_version_ids(
    client: &reqwest::Client,
    token: &str,
    mc: &str,
    loader: &str,
) -> Result<Vec<i64>> {
    let types = cf_list(
        client,
        token,
        "https://minecraft.curseforge.com/api/game/version-types",
    )
    .await?;
    let versions = cf_list(
        client,
        token,
        "https://minecraft.curseforge.com/api/game/versions",
    )
    .await?;
    game_version_ids(&types, &versions, mc, loader)
}

fn game_version_ids(
    types: &[serde_json::Value],
    versions: &[serde_json::Value],
    mc: &str,
    loader: &str,
) -> Result<Vec<i64>> {
    let type_ids = |keep: &dyn Fn(&str) -> bool| -> HashSet<i64> {
        types
            .iter()
            .filter(|t| t["slug"].as_str().is_some_and(keep))
            .filter_map(|t| t["id"].as_i64())
            .collect()
    };
    let minecraft = type_ids(&|slug| slug.starts_with("minecraft"));
    let modloader = type_ids(&|slug| slug == "modloader");
    let loader_name = match loader {
        "fabric" => "Fabric",
        "forge" => "Forge",
        "quilt" => "Quilt",
        "neoforge" => "NeoForge",
        _ => "",
    };
    let mut game = None;
    let mut ids = Vec::new();
    for v in versions {
        let (Some(id), Some(kind)) = (v["id"].as_i64(), v["gameVersionTypeID"].as_i64()) else {
            continue;
        };
        let name = v["name"].as_str().unwrap_or("");
        if name == mc && minecraft.contains(&kind) {
            game = Some(id);
        } else if !loader_name.is_empty() && name == loader_name && modloader.contains(&kind) {
            ids.push(id);
        }
    }
    let Some(game) = game else {
        bail!("CurseForge doesn't list Minecraft {mc}.");
    };
    ids.insert(0, game);
    Ok(ids)
}

fn file_safe(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "pack".into()
    } else {
        cleaned
    }
}

fn truncate(s: &str) -> String {
    let t = s.trim();
    match t.char_indices().nth(280) {
        Some((end, _)) => format!("{}…", &t[..end]),
        None => t.to_string(),
    }
}
