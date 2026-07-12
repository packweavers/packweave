use anyhow::{anyhow, Result};
use serde_json::json;

pub async fn create_release(
    repo: &str,
    token: &str,
    tag: &str,
    name: &str,
    body: &str,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("packweave/0.1.0")
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
    Ok(value
        .get("html_url")
        .and_then(|u| u.as_str())
        .unwrap_or_default()
        .to_string())
}
