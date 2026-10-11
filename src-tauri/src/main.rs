#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bulkadd;
mod cache;
mod cfpack;
mod commands;
mod configs;
mod content;
mod curseforge;
mod dist;
mod dropped;
mod fsbrowse;
mod git;
mod github;
mod gitsetup;
mod import;
mod instance;
mod launch;
mod launchers;
mod lenient;
mod lockfile;
#[cfg(target_os = "macos")]
mod macos;
mod manifest;
mod modmeta;
mod modrinth;
mod mrpack;
mod nbt;
mod packdiff;
mod packignore;
mod packlocal;
mod packwiz;
mod pathsafe;
mod preserve;
mod providers;
mod ptype;
mod publish;
mod release;
mod resolver;
mod search;
mod secrets;
mod sync;
mod syncbase;
mod syncignore;
mod undo;
mod util;
mod validate;
mod worktree;

use curseforge::CurseForge;
use modrinth::Modrinth;
use tauri::Manager;

pub const USER_AGENT: &str = concat!(
    "packweave/",
    env!("CARGO_PKG_VERSION"),
    " (modpack builder; +https://github.com/packweavers/packweave)"
);

#[cfg(target_os = "linux")]
fn dmabuf_is_broken() -> bool {
    use std::path::Path;
    if Path::new("/sys/module/nvidia").exists() || Path::new("/proc/driver/nvidia/version").exists()
    {
        return true;
    }
    if std::env::var_os("LIBGL_ALWAYS_SOFTWARE").is_some() {
        return true;
    }
    [
        "/sys/class/dmi/id/product_name",
        "/sys/class/dmi/id/sys_vendor",
    ]
    .iter()
    .filter_map(|p| std::fs::read_to_string(p).ok())
    .any(|s| {
        let s = s.to_lowercase();
        ["virtualbox", "innotek", "vmware", "qemu", "parallels"]
            .iter()
            .any(|v| s.contains(v))
    })
}

#[cfg(target_os = "linux")]
fn dbus_session_available() -> bool {
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() {
        return true;
    }
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(|d| std::path::Path::new(&d).join("bus").exists())
        .unwrap_or(false)
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(base) = app.path().data_dir() {
        let root = base.join("packweave");
        let cache = root.join("cache");
        let _ = std::fs::create_dir_all(&cache);
        app.state::<Modrinth>().set_cache_dir(cache);
        lockfile::set_cache_dir(root.join("lockcache"));
        syncbase::set_dir(root.join("syncbase"));
        syncignore::set_dir(root.join("syncignore"));
        worktree::set_dir(root.join("branches"));
        secrets::set_fallback_dir(root);
    }

    #[cfg(target_os = "macos")]
    macos::follow_fullscreen();
    if let Some(main) = app.get_webview_window(commands::MAIN_LABEL) {
        commands::prepare_window(&main);
    }
    Ok(())
}

fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() && dmabuf_is_broken() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let modrinth = Modrinth::new().expect("failed to build HTTP client");
    let curseforge = CurseForge::new().expect("failed to build HTTP client");

    #[cfg(target_os = "linux")]
    let single_instance = dbus_session_available();
    #[cfg(not(target_os = "linux"))]
    let single_instance = true;

    let mut builder = tauri::Builder::default();
    if single_instance {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window(commands::MAIN_LABEL) {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(setup)
        .manage(modrinth)
        .manage(curseforge)
        .manage(commands::Windows::default())
        .invoke_handler(tauri::generate_handler![
            commands::window_pack,
            commands::window_attach,
            commands::open_pack_window,
            commands::detach_tab,
            commands::tab_drop_target,
            commands::send_tab,
            commands::show_doc,
            commands::main_window,
            commands::open_settings_window,
            commands::pack_windows,
            commands::close_pack_window,
            commands::create_pack,
            commands::open_pack,
            commands::read_manifest,
            commands::pin_loader_version,
            commands::import_pack,
            commands::packwiz_info,
            commands::convert_packwiz,
            commands::clone_pack,
            commands::save_manifest,
            commands::set_datapack_folder,
            commands::preserve_patterns,
            commands::preserve_update,
            commands::preserve_reset,
            commands::list_providers,
            commands::search,
            commands::project_details,
            commands::version_changelog,
            commands::update_changelogs,
            commands::convert_search,
            commands::convert_lookup,
            commands::enrich_mods,
            commands::mod_versions,
            commands::get_loader_versions,
            commands::get_minecraft_versions,
            commands::add_content,
            commands::bulk_lookup,
            commands::identify_dropped,
            commands::add_dropped,
            commands::recheck_dropped,
            commands::remove_content,
            commands::set_content_disabled,
            commands::set_content_pinned,
            commands::set_content_bundled,
            commands::set_source_version,
            commands::set_content_explicit,
            commands::set_content_versions,
            commands::add_alt_sources,
            commands::set_preferred_source,
            commands::remove_alt_source,
            commands::resolve_pack,
            commands::preview_change,
            commands::check_updates,
            commands::list_unpublished,
            commands::undo,
            commands::redo,
            commands::undo_state,
            commands::bind_instance,
            commands::unbind_instance,
            commands::get_binding,
            commands::sync_status,
            commands::apply_sync,
            commands::sync_file_diff,
            commands::sync_ignore,
            commands::sync_unignore,
            commands::instance_jars,
            commands::set_instance_jars,
            commands::auto_push_file,
            commands::read_ignore,
            commands::write_ignore,
            commands::default_ignore,
            commands::add_ignore_patterns,
            commands::detect_instances,
            commands::resolve_instance_folder,
            commands::instance_running,
            commands::read_game_log,
            commands::create_prism_instance,
            commands::can_copy_instance,
            commands::copy_instance,
            commands::delete_instance,
            commands::pack_icon,
            commands::set_pack_icon,
            commands::export_mrpack,
            commands::export_curseforge,
            commands::export_instance,
            commands::release_folder,
            commands::include_packweaver,
            commands::release_checkout,
            commands::git_upstream_head,
            commands::git_mark_tag,
            commands::release_checkout_remove,
            commands::export_mrpack_selfupdate,
            commands::export_curseforge_selfupdate,
            commands::export_dist,
            commands::publish_bundling,
            commands::publish_pack,
            commands::fs_list,
            commands::fs_read,
            commands::fs_write,
            commands::fs_mkdir,
            commands::fs_delete,
            commands::fs_rename,
            commands::fs_read_image,
            commands::fs_read_nbt,
            commands::fs_index,
            commands::mod_configs,
            commands::search_files,
            commands::replace_in_files,
            commands::git_status,
            commands::git_init,
            commands::git_setup_state,
            commands::git_token_save,
            commands::git_token_delete,
            commands::git_setup_recheck,
            commands::git_setup_install,
            commands::git_set_identity,
            commands::git_commit,
            commands::git_log,
            commands::git_graph,
            commands::git_push,
            commands::git_pull,
            commands::git_fetch,
            commands::git_discard_files,
            commands::git_diff_file,
            commands::git_working_stats,
            commands::git_branches,
            commands::git_checkout,
            commands::git_create_branch,
            commands::git_rename_branch,
            commands::git_delete_branch,
            commands::git_delete_remote_branch,
            commands::git_rebase,
            commands::git_push_branch,
            commands::git_merge,
            commands::git_merge_state,
            commands::git_conflict_sides,
            commands::git_resolve_conflict,
            commands::git_resolve_with,
            commands::git_resolve_content_version,
            commands::git_merge_abort,
            commands::git_merge_commit,
            commands::git_compare,
            commands::git_commit_changes,
            commands::git_show_diff,
            commands::git_show_file,
            commands::git_pack_diff,
            commands::git_pack_diff_working,
            commands::git_latest_tag,
            commands::changelog_save,
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
            commands::git_github_repo,
            commands::git_remotes,
            commands::git_add_remote,
            commands::git_set_remote_url,
            commands::git_remove_remote,
            commands::worktree_list,
            commands::worktree_main,
            commands::branch_folder,
            commands::github_access,
            commands::github_release,
            commands::secret_set,
            commands::secret_delete,
            commands::secret_status,
            commands::cache_size,
            commands::clear_cache,
            commands::pack_dist_token,
            commands::save_pack_dist_token,
            commands::check_pack_dist_token,
            commands::read_prefs,
            commands::write_prefs,
            commands::app_info,
            commands::check_update,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
