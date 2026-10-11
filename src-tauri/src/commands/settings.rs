use serde::Serialize;
use std::path::Path;

use crate::secrets;

use super::es;
use ts_rs::TS;

#[tauri::command]
pub async fn secret_set(key: String, value: String) -> Result<(), String> {
    secrets::set(&key, &value).map_err(es)
}

#[tauri::command]
pub async fn secret_delete(key: String) -> Result<(), String> {
    secrets::delete(&key).map_err(es)
}

#[tauri::command]
pub async fn secret_status(keys: Vec<String>) -> Result<Vec<bool>, String> {
    Ok(keys
        .iter()
        .map(|k| secrets::get(k).is_some_and(|v| !v.trim().is_empty()))
        .collect())
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub platform: String,
    pub can_update: bool,
}

#[tauri::command]
pub async fn app_info(app: tauri::AppHandle) -> Result<AppInfo, String> {
    use tauri::Manager;
    let dir = app.path().data_dir().map_err(es)?.join("packweave");
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        data_dir: dir.to_string_lossy().to_string(),
        platform: std::env::consts::OS.into(),
        can_update: updater_supported(),
    })
}

#[tauri::command]
pub async fn read_prefs(app: tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let path = app
        .path()
        .data_dir()
        .map_err(es)?
        .join("packweave")
        .join("prefs.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok("{}".into()),
        Err(e) => return Err(format!("Couldn't read your settings: {e}")),
    };
    match crate::lenient::parse(&text) {
        Some((value, false)) if value.is_object() => Ok(text),
        Some((value, true)) if value.is_object() => {
            let fixed = serde_json::to_string_pretty(&value).map_err(es)?;
            let _ = std::fs::write(&path, &fixed);
            Ok(fixed)
        }
        _ => {
            std::fs::rename(&path, path.with_file_name("prefs.broken.json")).map_err(es)?;
            Err("Couldn't read your settings, so packweave started with the defaults. The old file is saved as prefs.broken.json.".into())
        }
    }
}

#[tauri::command]
pub async fn write_prefs(app: tauri::AppHandle, content: String) -> Result<(), String> {
    use tauri::Manager;
    let dir = app.path().data_dir().map_err(es)?.join("packweave");
    std::fs::create_dir_all(&dir).map_err(es)?;
    std::fs::write(dir.join("prefs.json"), content).map_err(es)
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
}

fn updater_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("APPIMAGE").is_some()
            || (tauri::utils::platform::bundle_type()
                == Some(tauri::utils::config::BundleType::Deb)
                && std::path::Path::new("/var/lib/dpkg/info/packweave.list").exists())
    }

    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    use tauri_plugin_updater::UpdaterExt;
    if !updater_supported() {
        return Ok(None);
    }
    let updater = app.updater().map_err(es)?;
    match updater.check().await.map_err(es)? {
        Some(u) => Ok(Some(UpdateInfo {
            version: u.version.clone(),
            notes: u.body.clone().unwrap_or_default(),
        })),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    if !updater_supported() {
        return Err("Automatic updates aren't available for this install. \
             Update packweave through your package manager."
            .into());
    }
    let updater = app.updater().map_err(es)?;
    let update = updater
        .check()
        .await
        .map_err(es)?
        .ok_or_else(|| "No update available.".to_string())?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(es)?;
    app.restart()
}

#[tauri::command]
pub async fn pack_dist_token(path: String) -> Result<Option<String>, String> {
    super::blocking(move || Ok(super::saved_dist_token(Path::new(&path)))).await
}

#[tauri::command]
pub async fn check_pack_dist_token(path: String, repo: String) -> Result<Option<bool>, String> {
    let saved = super::blocking(move || Ok(super::saved_dist_token(Path::new(&path)))).await?;
    let Some(token) = saved else {
        return Ok(None);
    };
    Ok(crate::github::token_can_read(&repo, &token).await)
}

#[tauri::command]
pub async fn save_pack_dist_token(path: String, token: String) -> Result<(), String> {
    super::blocking(move || {
        let key = super::dist_token_key(Path::new(&path));
        match token.trim() {
            "" => secrets::delete(&key),
            token => secrets::set(&key, token),
        }
    })
    .await
}

fn folder_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| {
            let path = e.path();
            if path.is_dir() {
                folder_size(&path)
            } else {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

fn cache_folders(app: &tauri::AppHandle) -> Vec<std::path::PathBuf> {
    use tauri::Manager;
    let mut out = vec![std::env::temp_dir().join("packweave-release")];
    if let Ok(base) = app.path().data_dir() {
        let root = base.join("packweave");
        out.push(root.join("cache"));
        out.push(root.join("lockcache"));
    }
    out
}

#[tauri::command]
pub async fn cache_size(app: tauri::AppHandle) -> Result<u64, String> {
    let folders = cache_folders(&app);
    super::blocking(move || Ok(folders.iter().map(|f| folder_size(f)).sum())).await
}

#[tauri::command]
pub async fn clear_cache(
    app: tauri::AppHandle,
    mr: tauri::State<'_, crate::modrinth::Modrinth>,
) -> Result<u64, String> {
    let folders = cache_folders(&app);
    let freed = super::blocking(move || {
        let freed = folders.iter().map(|f| folder_size(f)).sum();
        match std::fs::remove_dir_all(&folders[0]) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(freed),
        }
    })
    .await?;
    mr.forget_cached()
        .map_err(|e| format!("Couldn't clear the cache: {e}"))?;
    crate::lockfile::clear_cache().map_err(|e| format!("Couldn't clear the cache: {e}"))?;
    crate::instance::clear_hash_cache();
    Ok(freed)
}
