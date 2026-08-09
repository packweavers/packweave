use anyhow::{anyhow, Result};
use serde_json::json;

pub struct Release {
    pub html_url: String,
    pub id: u64,
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
    let url = format!(
        "https://uploads.github.com/repos/{repo}/releases/{release_id}/assets?name={name}"
    );
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/zip")
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
        .json(&json!({
            "tag_name": tag,
            "name": name,
            "body": body,
            "draft": false,
            "prerelease": false,
        }))
        .send()
        .await?;
    let status = resp.status();
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    if !status.is_success() {
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
