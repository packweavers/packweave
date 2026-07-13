use anyhow::{bail, Result};
use serde_json::json;
use std::path::Path;

use crate::cfpack;
use crate::curseforge::CurseForge;
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::mrpack;

const USER_AGENT: &str = "packweave/0.1.0";

#[allow(clippy::too_many_arguments)]
pub async fn publish_modrinth(
    lock: &Lockfile,
    manifest: &Manifest,
    overrides: &Path,
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
    let tmp = std::env::temp_dir()
        .join(format!("packweave-{}.mrpack", sanitize(version_number)));
    mrpack::export(lock, &manifest.name, version_number, &tmp, env, overrides)
        .await?;
    let bytes = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);

    let mut loaders: Vec<String> = Vec::new();
    if manifest.loader != "vanilla" {
        loaders.push(manifest.loader.clone());
    }
    let data = json!({
        "name": version_number,
        "version_number": version_number,
        "changelog": changelog,
        "dependencies": [],
        "game_versions": [manifest.minecraft],
        "version_type": "release",
        "loaders": loaders,
        "featured": true,
        "status": "listed",
        "project_id": project_id,
        "file_parts": ["file"],
        "primary_file": "file",
    });

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name("pack.mrpack")
        .mime_str("application/x-modrinth-modpack+zip")?;
    let form = reqwest::multipart::Form::new()
        .text("data", serde_json::to_string(&data)?)
        .part("file", part);

    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
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
    let v: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|_| json!({}));
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
    overrides: &Path,
    project_id: &str,
    version_number: &str,
    changelog: &str,
    env: &str,
    token: &str,
) -> Result<String> {
    if token.is_empty() {
        bail!("Add your CurseForge upload token in Settings first.");
    }
    if project_id.is_empty() {
        bail!("Enter the CurseForge project id.");
    }
    let tmp = std::env::temp_dir()
        .join(format!("packweave-{}-cf.zip", sanitize(version_number)));
    cfpack::export(
        cf,
        lock,
        &manifest.name,
        version_number,
        &tmp,
        env,
        overrides,
    )
    .await?;
    let bytes = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);

    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
    let game_versions = cf_game_version_ids(
        &client,
        token,
        &manifest.minecraft,
        &manifest.loader,
    )
    .await
    .unwrap_or_default();
    let metadata = json!({
        "changelog": changelog,
        "changelogType": "text",
        "releaseType": "release",
        "gameVersions": game_versions,
    });

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(format!("{}.zip", sanitize(&manifest.name)))
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
    let v: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|_| json!({}));
    let id = v["id"].as_i64().map(|n| n.to_string()).unwrap_or_default();
    Ok(format!(
        "Uploaded to CurseForge project {project_id} (file {id})."
    ))
}

async fn cf_game_version_ids(
    client: &reqwest::Client,
    token: &str,
    mc: &str,
    loader: &str,
) -> Result<Vec<i64>> {
    let resp = client
        .get("https://minecraft.curseforge.com/api/game/versions")
        .header("X-Api-Token", token)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let arr: Vec<serde_json::Value> = resp.json().await?;
    let loader_name = match loader {
        "fabric" => "Fabric",
        "forge" => "Forge",
        "quilt" => "Quilt",
        "neoforge" => "NeoForge",
        _ => "",
    };
    let mut ids = Vec::new();
    for v in &arr {
        let name = v["name"].as_str().unwrap_or("");
        if name == mc || (!loader_name.is_empty() && name == loader_name) {
            if let Some(id) = v["id"].as_i64() {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}

fn sanitize(s: &str) -> String {
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
    if t.len() > 280 {
        format!("{}…", &t[..280])
    } else {
        t.to_string()
    }
}
