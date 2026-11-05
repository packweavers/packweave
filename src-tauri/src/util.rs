use std::path::{Path, PathBuf};

pub fn real_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn same_dir(a: &Path, b: &Path) -> bool {
    real_path(a) == real_path(b)
}

pub fn is_false(value: &bool) -> bool {
    !*value
}

pub fn is_zero<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

pub fn required() -> String {
    "required".into()
}

pub fn failure(error: &reqwest::Error) -> String {
    if let Some(status) = error.status() {
        format!("the server returned {status}")
    } else if error.is_timeout() {
        "the connection timed out".into()
    } else if error.is_connect() {
        "couldn't connect".into()
    } else {
        "the connection dropped".into()
    }
}

pub fn download_error(name: &str, error: &reqwest::Error) -> anyhow::Error {
    anyhow::anyhow!("Couldn't download {name}: {}", failure(error))
}

pub async fn download(client: &reqwest::Client, url: &str, name: &str) -> anyhow::Result<Vec<u8>> {
    let response = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| download_error(name, &e))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| download_error(name, &e))?;
    Ok(bytes.to_vec())
}

pub fn delete_file(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

pub fn delete_dir_all(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

pub fn write_file(path: &Path, contents: impl AsRef<[u8]>) -> anyhow::Result<()> {
    std::fs::write(path, contents).map_err(|e| {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        anyhow::anyhow!("Couldn't write {name}: {e}")
    })
}
