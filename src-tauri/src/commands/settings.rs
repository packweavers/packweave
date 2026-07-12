use serde::Serialize;

use crate::secrets;

use super::es;

#[tauri::command]
pub async fn secret_set(key: String, value: String) -> Result<(), String> {
    secrets::set(&key, &value).map_err(es)
}

#[tauri::command]
pub async fn secret_delete(key: String) -> Result<(), String> {
    secrets::delete(&key).map_err(es)
}

#[tauri::command]
pub async fn read_prefs(app: tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let dir = app.path().data_dir().map_err(es)?.join("packweave");
    Ok(std::fs::read_to_string(dir.join("prefs.json"))
        .unwrap_or_else(|_| "{}".into()))
}

#[tauri::command]
pub async fn write_prefs(
    app: tauri::AppHandle,
    content: String,
) -> Result<(), String> {
    use tauri::Manager;
    let dir = app.path().data_dir().map_err(es)?.join("packweave");
    std::fs::create_dir_all(&dir).map_err(es)?;
    std::fs::write(dir.join("prefs.json"), content).map_err(es)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
}

#[tauri::command]
pub async fn check_update(
    app: tauri::AppHandle,
) -> Result<Option<UpdateInfo>, String> {
    use tauri_plugin_updater::UpdaterExt;
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
