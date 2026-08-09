use anyhow::Result;
use std::path::PathBuf;
use std::sync::OnceLock;

const SERVICE: &str = "packweave";

static FALLBACK_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_fallback_dir(dir: PathBuf) {
    let _ = FALLBACK_DIR.set(dir);
}

fn fallback_file() -> Option<PathBuf> {
    FALLBACK_DIR.get().map(|d| d.join("secrets.json"))
}

fn read_fallback() -> serde_json::Map<String, serde_json::Value> {
    fallback_file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_fallback(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<()> {
    let Some(path) = fallback_file() else {
        anyhow::bail!("secret storage is unavailable");
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if map.is_empty() {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    std::fs::write(&path, serde_json::to_string(map)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o600),
        );
    }
    Ok(())
}

fn fallback_get(key: &str) -> Option<String> {
    read_fallback()
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn fallback_set(key: &str, value: &str) -> Result<()> {
    let mut map = read_fallback();
    map.insert(key.to_string(), serde_json::Value::String(value.into()));
    write_fallback(&map)
}

fn fallback_delete(key: &str) {
    let mut map = read_fallback();
    if map.remove(key).is_some() {
        let _ = write_fallback(&map);
    }
}

pub fn set(key: &str, value: &str) -> Result<()> {
    match keyring::Entry::new(SERVICE, key)
        .and_then(|entry| entry.set_password(value))
    {
        Ok(()) => {
            fallback_delete(key);
            Ok(())
        }
        Err(keyring_err) => fallback_set(key, value).map_err(|_| {
            anyhow::anyhow!(
                "couldn't store the token in the system keychain: {keyring_err}"
            )
        }),
    }
}

pub fn get(key: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, key)
        .ok()
        .and_then(|entry| entry.get_password().ok())
        .or_else(|| fallback_get(key))
}

pub fn delete(key: &str) -> Result<()> {
    if let Ok(entry) = keyring::Entry::new(SERVICE, key) {
        let _ = entry.delete_credential();
    }
    fallback_delete(key);
    Ok(())
}
