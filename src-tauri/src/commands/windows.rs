use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, EventTarget, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use super::es;
use ts_rs::TS;

pub const MAIN_LABEL: &str = "main";
pub const SETTINGS_LABEL: &str = "settings";
const PANEL_KINDS: [&str; 3] = ["problems", "output", "gamelog"];

struct PackWindow {
    dir: PathBuf,
    detached: bool,
}

#[derive(Default)]
pub struct Windows {
    packs: Mutex<HashMap<String, PackWindow>>,
    pending_tabs: Mutex<HashMap<String, TabPayload>>,
    focus_order: Mutex<Vec<String>>,
    next: Mutex<u32>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct OpenWindow {
    pub label: String,
    pub dir: String,
}

#[derive(Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct TabBuffer {
    pub original: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct TabPayload {
    pub kind: String,
    pub arg: String,
    pub pinned: bool,
    pub buffer: Option<TabBuffer>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct WindowPack {
    pub dir: String,
    pub detached: bool,
    pub tab: Option<TabPayload>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "types.ts")]
pub struct DropTarget {
    pub label: String,
    pub same_branch: bool,
}

impl Windows {
    fn primary(&self, dir: &Path) -> Option<String> {
        let packs = self.packs.lock().ok()?;
        packs
            .iter()
            .find(|(_, w)| !w.detached && crate::util::same_dir(&w.dir, dir))
            .map(|(label, _)| label.clone())
    }

    fn labels_for(&self, dir: &Path) -> Vec<String> {
        let Ok(packs) = self.packs.lock() else {
            return Vec::new();
        };
        packs
            .iter()
            .filter(|(_, w)| crate::util::same_dir(&w.dir, dir))
            .map(|(label, _)| label.clone())
            .collect()
    }

    fn register(&self, label: &str, dir: PathBuf, detached: bool) {
        if let Ok(mut packs) = self.packs.lock() {
            packs.insert(label.to_string(), PackWindow { dir, detached });
        }
    }

    fn forget(&self, label: &str) {
        if let Ok(mut packs) = self.packs.lock() {
            packs.remove(label);
        }
        if let Ok(mut pending) = self.pending_tabs.lock() {
            pending.remove(label);
        }
        if let Ok(mut order) = self.focus_order.lock() {
            order.retain(|l| l != label);
        }
    }

    fn dir_of(&self, label: &str) -> Option<PathBuf> {
        self.packs.lock().ok()?.get(label).map(|w| w.dir.clone())
    }

    fn is_detached(&self, label: &str) -> bool {
        self.packs
            .lock()
            .ok()
            .and_then(|packs| packs.get(label).map(|w| w.detached))
            .unwrap_or(false)
    }

    fn detached_from(&self, label: &str) -> Vec<String> {
        let Ok(packs) = self.packs.lock() else {
            return Vec::new();
        };
        let Some(primary) = packs.get(label).filter(|w| !w.detached) else {
            return Vec::new();
        };
        packs
            .iter()
            .filter(|(_, w)| w.detached && crate::util::same_dir(&w.dir, &primary.dir))
            .map(|(other, _)| other.clone())
            .collect()
    }

    fn take_tab(&self, label: &str) -> Option<TabPayload> {
        self.pending_tabs.lock().ok()?.remove(label)
    }

    fn focused(&self, label: &str) {
        if let Ok(mut order) = self.focus_order.lock() {
            order.retain(|l| l != label);
            order.insert(0, label.to_string());
        }
    }

    fn by_focus(&self) -> Vec<String> {
        self.focus_order
            .lock()
            .map(|order| order.clone())
            .unwrap_or_default()
    }

    fn next_label(&self) -> String {
        let mut next = self.next.lock().unwrap_or_else(|e| e.into_inner());
        *next += 1;
        format!("pack-{next}")
    }
}

pub fn prepare_window(window: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    crate::macos::style_window(window);
    let app = window.app_handle().clone();
    let label = window.label().to_string();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Focused(true) => app.state::<Windows>().focused(&label),
        tauri::WindowEvent::Destroyed => {
            let windows = app.state::<Windows>();
            let detached = windows.detached_from(&label);
            windows.forget(&label);
            let _ = app.emit("window-closed", label.clone());
            for other in detached {
                if let Some(window) = app.get_webview_window(&other) {
                    let _ = window.close();
                }
            }
        }
        _ => {}
    });
}

fn focus(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn contains(window: &WebviewWindow, x: f64, y: f64) -> bool {
    if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(true) {
        return false;
    }
    let scale = window.scale_factor().unwrap_or(1.0);
    let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) else {
        return false;
    };
    let position = position.to_logical::<f64>(scale);
    let size = size.to_logical::<f64>(scale);
    x >= position.x
        && x <= position.x + size.width
        && y >= position.y
        && y <= position.y + size.height
}

struct Placement {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn centred(window: &WebviewWindow, width: f64, height: f64) -> (f64, f64) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        let at = area.position.to_logical::<f64>(scale);
        let size = area.size.to_logical::<f64>(scale);
        return (
            at.x + (size.width - width).max(0.0) / 2.0,
            at.y + (size.height - height).max(0.0) / 2.0,
        );
    }
    let scale = window.scale_factor().unwrap_or(1.0);
    match (window.outer_position(), window.outer_size()) {
        (Ok(at), Ok(size)) => {
            let at = at.to_logical::<f64>(scale);
            let size = size.to_logical::<f64>(scale);
            (
                at.x + (size.width - width) / 2.0,
                at.y + (size.height - height) / 2.0,
            )
        }
        _ => (120.0, 120.0),
    }
}

fn build_window(
    app: &AppHandle,
    label: &str,
    title: &str,
    query: &str,
    placement: Option<Placement>,
) -> Result<WebviewWindow, String> {
    let url = WebviewUrl::App(format!("index.html{query}").into());
    let builder = WebviewWindowBuilder::new(app, label, url).title(title);
    let builder = match placement {
        Some(p) => builder
            .inner_size(p.width, p.height)
            .min_inner_size(p.width.min(480.0), p.height.min(320.0))
            .position(p.x, p.y),
        None => builder
            .inner_size(1320.0, 860.0)
            .min_inner_size(980.0, 640.0),
    };
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.decorations(false).visible(false);
    let window = builder.build().map_err(es)?;
    let w = window.clone();
    let _ = app.run_on_main_thread(move || prepare_window(&w));
    Ok(window)
}

#[tauri::command]
pub async fn window_pack(
    window: WebviewWindow,
    windows: State<'_, Windows>,
) -> Result<Option<WindowPack>, String> {
    let label = window.label();
    Ok(windows.dir_of(label).map(|dir| WindowPack {
        dir: dir.to_string_lossy().to_string(),
        detached: windows.is_detached(label),
        tab: windows.take_tab(label),
    }))
}

#[tauri::command]
pub async fn window_attach(
    app: AppHandle,
    window: WebviewWindow,
    windows: State<'_, Windows>,
    dir: Option<String>,
    title: String,
) -> Result<Option<String>, String> {
    let _ = window.set_title(&title);
    let label = window.label();
    let Some(dir) = dir.map(PathBuf::from) else {
        windows.forget(label);
        return Ok(None);
    };
    if windows.is_detached(label) {
        return Ok(None);
    }
    if let Some(other) = windows.primary(&dir).filter(|l| l != label) {
        if let Some(existing) = app.get_webview_window(&other) {
            focus(&existing);
            return Ok(Some(other));
        }
    }
    windows.register(label, dir, false);
    Ok(None)
}

#[tauri::command]
pub async fn open_pack_window(
    app: AppHandle,
    windows: State<'_, Windows>,
    dir: String,
    title: String,
) -> Result<String, String> {
    let dir = PathBuf::from(dir);
    if let Some(label) = windows.primary(&dir) {
        if let Some(existing) = app.get_webview_window(&label) {
            focus(&existing);
            return Ok(label);
        }
    }
    let label = windows.next_label();
    windows.register(&label, dir, false);
    match build_window(&app, &label, &title, "", None) {
        Ok(_) => Ok(label),
        Err(e) => {
            windows.forget(&label);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn detach_tab(
    app: AppHandle,
    window: WebviewWindow,
    windows: State<'_, Windows>,
    tab: TabPayload,
    x: Option<f64>,
    y: Option<f64>,
) -> Result<String, String> {
    let dir = windows
        .dir_of(window.label())
        .ok_or_else(|| "This window has no pack open.".to_string())?;
    let title = window.title().unwrap_or_default();
    let panel = PANEL_KINDS.contains(&tab.kind.as_str());
    let (width, height) = if panel {
        (760.0, 460.0)
    } else {
        (900.0, 640.0)
    };
    let (x, y) = match (x, y) {
        (Some(x), Some(y)) => (x - 160.0, y - 18.0),
        _ => centred(&window, width, height),
    };
    let label = windows.next_label();
    windows.register(&label, dir, true);
    if let Ok(mut pending) = windows.pending_tabs.lock() {
        pending.insert(label.clone(), tab);
    }
    let placement = Placement {
        x,
        y,
        width,
        height,
    };
    match build_window(&app, &label, &title, "", Some(placement)) {
        Ok(_) => Ok(label),
        Err(e) => {
            windows.forget(&label);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn tab_drop_target(
    app: AppHandle,
    window: WebviewWindow,
    windows: State<'_, Windows>,
    x: f64,
    y: f64,
) -> Result<Option<DropTarget>, String> {
    let me = window.label().to_string();
    let mine = windows.dir_of(&me);
    let mut labels = windows.by_focus();
    for label in app.webview_windows().into_keys() {
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    for label in labels {
        let Some(other) = app.get_webview_window(&label) else {
            continue;
        };
        if !contains(&other, x, y) {
            continue;
        }
        let Some(dir) = windows.dir_of(&label).filter(|_| label != me) else {
            return Ok(None);
        };
        let same_branch = mine
            .as_deref()
            .is_some_and(|m| crate::util::same_dir(m, &dir));
        return Ok(Some(DropTarget { label, same_branch }));
    }
    Ok(None)
}

#[derive(Serialize, Clone, TS)]
#[ts(export, export_to = "types.ts")]
pub struct IncomingTab {
    tab: TabPayload,
    x: Option<f64>,
    y: Option<f64>,
}

#[tauri::command]
pub async fn send_tab(
    app: AppHandle,
    label: String,
    tab: TabPayload,
    x: Option<f64>,
    y: Option<f64>,
) -> Result<(), String> {
    let target = app
        .get_webview_window(&label)
        .ok_or_else(|| "That window is gone.".to_string())?;
    app.emit_to(
        EventTarget::webview_window(&label),
        "receive-tab",
        IncomingTab { tab, x, y },
    )
    .map_err(es)?;
    focus(&target);
    Ok(())
}

#[tauri::command]
pub async fn show_doc(app: AppHandle, label: String, id: String) -> Result<(), String> {
    let target = app
        .get_webview_window(&label)
        .ok_or_else(|| "That window is gone.".to_string())?;
    app.emit_to(EventTarget::webview_window(&label), "show-doc", id)
        .map_err(es)?;
    focus(&target);
    Ok(())
}

#[tauri::command]
pub async fn main_window(
    window: WebviewWindow,
    windows: State<'_, Windows>,
) -> Result<Option<String>, String> {
    let me = window.label();
    Ok(windows
        .dir_of(me)
        .and_then(|dir| windows.primary(&dir))
        .filter(|label| label != me))
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle, page: Option<String>) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(SETTINGS_LABEL) {
        focus(&existing);
        if let Some(page) = page {
            let _ = app.emit_to(
                EventTarget::webview_window(SETTINGS_LABEL),
                "settings-page",
                page,
            );
        }
        return Ok(());
    }
    let query = match page {
        Some(p) => format!("?window=settings&page={p}"),
        None => "?window=settings".into(),
    };
    let window = build_window(&app, SETTINGS_LABEL, "Settings", &query, None)?;
    let _ = window.set_size(tauri::LogicalSize::new(880.0, 640.0));
    let _ = window.set_min_size(Some(tauri::LogicalSize::new(720.0, 480.0)));
    Ok(())
}

#[tauri::command]
pub async fn pack_windows(windows: State<'_, Windows>) -> Result<Vec<OpenWindow>, String> {
    let packs = windows.packs.lock().map_err(es)?;
    Ok(packs
        .iter()
        .map(|(label, w)| OpenWindow {
            label: label.clone(),
            dir: w.dir.to_string_lossy().to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn close_pack_window(
    app: AppHandle,
    windows: State<'_, Windows>,
    dir: String,
) -> Result<(), String> {
    for label in windows.labels_for(Path::new(&dir)) {
        let Some(window) = app.get_webview_window(&label) else {
            continue;
        };
        if label == MAIN_LABEL {
            let _ = app.emit_to(EventTarget::webview_window(MAIN_LABEL), "close-pack", ());
        } else {
            let _ = window.close();
        }
    }
    Ok(())
}
