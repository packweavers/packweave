#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bulkadd;
mod cache;
mod cfpack;
mod changelog;
mod commands;
mod content;
mod curseforge;
mod dist;
mod download;
mod dropped;
mod fsbrowse;
mod git;
mod github;
mod import;
mod instance;
mod launchers;
mod lockfile;
mod manifest;
mod modmeta;
mod modrinth;
mod mrpack;
mod nbt;
mod packdiff;
mod packlocal;
mod providers;
mod ptype;
mod publish;
mod resolver;
mod search;
mod secrets;
mod sync;
mod validate;

use curseforge::CurseForge;
use modrinth::Modrinth;

#[cfg(target_os = "macos")]
mod traffic_lights {
    use objc2::runtime::AnyObject;
    use objc2::{msg_send, Encode, Encoding};

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }
    unsafe impl Encode for CGPoint {
        const ENCODING: Encoding =
            Encoding::Struct("CGPoint", &[Encoding::Double, Encoding::Double]);
    }
    unsafe impl Encode for CGSize {
        const ENCODING: Encoding =
            Encoding::Struct("CGSize", &[Encoding::Double, Encoding::Double]);
    }
    unsafe impl Encode for CGRect {
        const ENCODING: Encoding =
            Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
    }

    const INSET_X: f64 = 18.0;
    const BAR_HEIGHT: f64 = 48.0;
    const FULLSCREEN_MASK: usize = 1 << 14;

    pub fn position(window: &tauri::WebviewWindow) {
        let Ok(ns_window) = window.ns_window() else {
            return;
        };
        let ns_window = ns_window as *mut AnyObject;
        unsafe {
            let style: usize = msg_send![ns_window, styleMask];
            if style & FULLSCREEN_MASK != 0 {
                return;
            }
            let close: *mut AnyObject =
                msg_send![ns_window, standardWindowButton: 0usize];
            let mini: *mut AnyObject =
                msg_send![ns_window, standardWindowButton: 1usize];
            let zoom: *mut AnyObject =
                msg_send![ns_window, standardWindowButton: 2usize];
            if close.is_null() || mini.is_null() || zoom.is_null() {
                return;
            }
            let close_frame: CGRect = msg_send![close, frame];
            let mini_frame: CGRect = msg_send![mini, frame];
            let spacing = mini_frame.origin.x - close_frame.origin.x;
            let container: *mut AnyObject = msg_send![close, superview];
            if container.is_null() {
                return;
            }
            let container_frame: CGRect = msg_send![container, frame];
            let top = (BAR_HEIGHT - close_frame.size.height) / 2.0;
            let y = container_frame.size.height - top - close_frame.size.height;
            for (i, button) in [close, mini, zoom].into_iter().enumerate() {
                let origin = CGPoint {
                    x: INSET_X + spacing * i as f64,
                    y,
                };
                let _: () = msg_send![button, setFrameOrigin: origin];
            }
        }
    }
}

fn main() {
    if std::env::var_os("PACKWEAVE_ASKPASS").is_some() {
        let prompt = std::env::args().nth(1).unwrap_or_default().to_lowercase();
        let answer = if prompt.starts_with("username") {
            std::env::var("PACKWEAVE_GIT_USER")
                .unwrap_or_else(|_| "x-access-token".into())
        } else {
            std::env::var("PACKWEAVE_GIT_TOKEN").unwrap_or_default()
        };
        println!("{answer}");
        return;
    }

    let modrinth = Modrinth::new().expect("failed to build HTTP client");
    let curseforge = CurseForge::new().expect("failed to build HTTP client");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            use tauri::Manager;
            if let Ok(base) = app.path().data_dir() {
                let cache = base.join("packweave").join("cache");
                let _ = std::fs::create_dir_all(&cache);
                app.state::<Modrinth>().set_cache_dir(cache);
                let lockcache = base.join("packweave").join("lockcache");
                let _ = std::fs::create_dir_all(&lockcache);
                lockfile::set_cache_dir(lockcache);
            }
            #[cfg(target_os = "macos")]
            if let Some(win) = app.get_webview_window("main") {
                traffic_lights::position(&win);
                let w = win.clone();
                win.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Resized(_)) {
                        traffic_lights::position(&w);
                    }
                });
            }
            Ok(())
        })
        .manage(modrinth)
        .manage(curseforge)
        .invoke_handler(tauri::generate_handler![
            commands::create_pack,
            commands::open_pack,
            commands::import_pack,
            commands::clone_pack,
            commands::save_manifest,
            commands::search,
            commands::list_providers,
            commands::add_mod,
            commands::add_content,
            commands::bulk_lookup,
            commands::add_content_bulk,
            commands::identify_dropped,
            commands::add_dropped,
            commands::remove_mod,
            commands::resolve_pack,
            commands::promote_mod,
            commands::set_content_disabled,
            commands::set_content_disabled_bulk,
            commands::remove_mods_bulk,
            commands::convert_search,
            commands::convert_lookup,
            commands::add_alt_source,
            commands::set_preferred_source,
            commands::remove_alt_source,
            commands::enrich_mods,
            commands::mod_versions,
            commands::set_mod_version,
            commands::set_mod_versions,
            commands::update_impact,
            commands::export_mrpack,
            commands::export_curseforge,
            commands::export_instance,
            commands::export_mrpack_selfupdate,
            commands::export_curseforge_selfupdate,
            commands::download_mods,
            commands::get_loader_versions,
            commands::get_minecraft_versions,
            commands::bind_instance,
            commands::unbind_instance,
            commands::get_binding,
            commands::sync_status,
            commands::apply_sync,
            commands::sync_file_diff,
            commands::auto_push_file,
            commands::git_status,
            commands::git_init,
            commands::git_commit,
            commands::git_log,
            commands::git_push,
            commands::git_pull,
            commands::git_discard,
            commands::read_gitignore,
            commands::write_gitignore,
            commands::detect_instances,
            commands::pack_icon,
            commands::set_pack_icon,
            commands::clear_pack_icon,
            commands::fs_list,
            commands::fs_read,
            commands::fs_write,
            commands::fs_mkdir,
            commands::fs_delete,
            commands::fs_rename,
            commands::fs_read_image,
            commands::fs_read_nbt,
            commands::search_files,
            commands::git_diff_file,
            commands::git_discard_file,
            commands::git_revert,
            commands::git_resolve_conflict,
            commands::git_branches,
            commands::git_checkout,
            commands::git_create_branch,
            commands::git_rename_branch,
            commands::git_delete_branch,
            commands::git_delete_remote_branch,
            commands::git_merge,
            commands::git_rebase,
            commands::git_set_upstream,
            commands::git_fetch,
            commands::git_push_branch,
            commands::git_commit_changes,
            commands::git_show_diff,
            commands::git_pack_diff,
            commands::git_pack_diff_working,
            commands::changelog_between,
            commands::changelog_working,
            commands::changelog_save,
            commands::changelog_head,
            commands::git_latest_tag,
            commands::git_revert_commit,
            commands::git_reset,
            commands::git_cherry_pick,
            commands::git_stash_list,
            commands::git_stash_push,
            commands::git_stash_apply,
            commands::git_stash_drop,
            commands::git_tags,
            commands::git_create_tag,
            commands::git_delete_tag,
            commands::git_push_tag,
            commands::git_pack_url,
            commands::git_remotes,
            commands::git_add_remote,
            commands::git_set_remote_url,
            commands::git_remove_remote,
            commands::github_release,
            commands::list_unpublished,
            commands::export_dist,
            commands::publish_pack,
            commands::secret_set,
            commands::secret_delete,
            commands::read_prefs,
            commands::write_prefs,
            commands::check_update,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
