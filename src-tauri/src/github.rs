use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::json;
use ts_rs::TS;

pub struct Release {
    pub html_url: String,
    pub id: u64,
}

pub fn provider_of(host: Option<&str>, token: &str) -> Option<&'static str> {
    if let Some(h) = host {
        let h = h.to_lowercase();
        if h.contains("github.com") {
            return Some("github");
        }
        if h.contains("gitlab.com") {
            return Some("gitlab");
        }
        return None;
    }
    if token.starts_with("ghp_") || token.starts_with("github_pat_") || token.starts_with("gho_") {
        return Some("github");
    }
    if token.starts_with("glpat-") {
        return Some("gitlab");
    }
    None
}

pub async fn verify_token(host: Option<&str>, token: &str) -> Result<Option<String>> {
    let token = token.trim();
    let Some(provider) = provider_of(host, token) else {
        return Ok(None);
    };
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()?;
    let (url, field) = match provider {
        "gitlab" => ("https://gitlab.com/api/v4/user", "username"),
        _ => ("https://api.github.com/user", "login"),
    };
    let request = if provider == "gitlab" {
        client.get(url).header("PRIVATE-TOKEN", token)
    } else {
        client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
    };
    let resp = request.send().await?;
    let label = if provider == "gitlab" {
        "GitLab"
    } else {
        "GitHub"
    };
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED
        || resp.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err(anyhow!("{label} rejected that token."));
    }
    if !resp.status().is_success() {
        return Err(anyhow!("couldn't reach {label} to check that token."));
    }
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    Ok(value
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()))
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "types.ts", rename = "RepoVisibility")]
pub enum Visibility {
    Public,
    Private,
    Missing,
    Unknown,
}

pub fn is_repo_slug(repo: &str) -> bool {
    let Some((owner, name)) = repo.split_once('/') else {
        return false;
    };
    let owner_ok =
        !owner.is_empty() && owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let name_ok = !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    owner_ok && name_ok
}

pub async fn repo_visibility(repo: &str, token: Option<&str>) -> Visibility {
    if !is_repo_slug(repo) {
        return Visibility::Missing;
    }
    let mut found = repo_request(repo, token).await;
    let rejected = matches!(&found, Ok(resp) if resp.status() == reqwest::StatusCode::UNAUTHORIZED);
    if rejected && token.is_some() {
        found = repo_request(repo, None).await;
    }
    let Ok(resp) = found else {
        return Visibility::Unknown;
    };
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Visibility::Missing;
    }
    if !resp.status().is_success() {
        return Visibility::Unknown;
    }
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    match value.get("private").and_then(|v| v.as_bool()) {
        Some(true) => Visibility::Private,
        Some(false) => Visibility::Public,
        None => Visibility::Unknown,
    }
}

pub async fn token_can_read(repo: &str, token: &str) -> Option<bool> {
    if !is_repo_slug(repo) {
        return Some(false);
    }
    let status = repo_request(repo, Some(token)).await.ok()?.status();
    if status.is_success() {
        return Some(true);
    }
    matches!(status.as_u16(), 401 | 403 | 404).then_some(false)
}

async fn repo_request(repo: &str, token: Option<&str>) -> Result<reqwest::Response> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()?;
    let mut request = client
        .get(format!("https://api.github.com/repos/{repo}"))
        .header("Accept", "application/vnd.github+json");
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {}", token.trim()));
    }
    Ok(request.send().await?)
}

pub async fn upload_asset(
    repo: &str,
    token: &str,
    release_id: u64,
    name: &str,
    bytes: Vec<u8>,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()?;
    let kind = match name.rsplit_once('.').map(|(_, ext)| ext) {
        Some("zip") => "application/zip",
        Some("mrpack") => "application/x-modrinth-modpack+zip",
        _ => "application/octet-stream",
    };
    let resp = client
        .post(format!(
            "https://uploads.github.com/repos/{repo}/releases/{release_id}/assets"
        ))
        .query(&[("name", name)])
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", kind)
        .body(bytes)
        .send()
        .await?;
    if !resp.status().is_success() {
        let value: serde_json::Value = resp.json().await.unwrap_or_default();
        let msg = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("asset upload failed");
        return Err(anyhow!("uploading {name}: {msg}"));
    }
    Ok(())
}

pub async fn create_release(
    repo: &str,
    token: &str,
    tag: &str,
    name: &str,
    body: &str,
    prerelease: bool,
    target: Option<&str>,
) -> Result<Release> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()?;
    let url = format!("https://api.github.com/repos/{repo}/releases");
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&{
            let mut payload = json!({
                "tag_name": tag,
                "name": name,
                "body": body,
                "draft": false,
                "prerelease": prerelease,
            });
            if let Some(target) = target {
                payload["target_commitish"] = json!(target);
            }
            payload
        })
        .send()
        .await?;
    let status = resp.status();
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    if !status.is_success() {
        let exists = value
            .get("errors")
            .and_then(|e| e.as_array())
            .is_some_and(|errors| {
                errors
                    .iter()
                    .any(|e| e.get("code").and_then(|c| c.as_str()) == Some("already_exists"))
            });
        if exists {
            return Err(anyhow!("{repo} already has a release for {tag}."));
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(anyhow!(
                "Couldn't find {repo}, or your GitHub sign-in can't create releases there."
            ));
        }
        let msg = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("release request failed");
        return Err(anyhow!("{msg}"));
    }
    Ok(Release {
        html_url: value
            .get("html_url")
            .and_then(|u| u.as_str())
            .unwrap_or_default()
            .to_string(),
        id: value.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
    })
}
