use std::path::PathBuf;
use tauri::State;

use crate::curseforge::CurseForge;
use crate::modrinth::Modrinth;
use crate::undo::{self, UndoState};

use super::{es, lock_pack, resolve_held, PackResolved};
use ts_rs::TS;

#[derive(serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct UndoResult {
    pub label: String,
    pub resolved: Option<PackResolved>,
}

#[tauri::command]
pub async fn undo(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
) -> Result<Option<UndoResult>, String> {
    let dir = PathBuf::from(&path);
    let guard = lock_pack(&dir).await;
    let Some(label) = undo::undo(&dir).map_err(es)? else {
        return Ok(None);
    };
    let resolved = resolve_held(&mr, &cf, &dir, &guard).await.ok();
    Ok(Some(UndoResult { label, resolved }))
}

#[tauri::command]
pub async fn redo(
    mr: State<'_, Modrinth>,
    cf: State<'_, CurseForge>,
    path: String,
) -> Result<Option<UndoResult>, String> {
    let dir = PathBuf::from(&path);
    let guard = lock_pack(&dir).await;
    let Some(label) = undo::redo(&dir).map_err(es)? else {
        return Ok(None);
    };
    let resolved = resolve_held(&mr, &cf, &dir, &guard).await.ok();
    Ok(Some(UndoResult { label, resolved }))
}

#[tauri::command]
pub async fn undo_state(path: String) -> Result<UndoState, String> {
    Ok(undo::state(&PathBuf::from(path)))
}
