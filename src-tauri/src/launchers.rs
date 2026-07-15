use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::dist;

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InstanceKind {
    Local,
    Modpack,
    Server,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedInstance {
    pub launcher: String,
    pub name: String,
    pub game_dir: String,
    pub minecraft: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub kind: InstanceKind,
    pub source: Option<String>,
    pub pack_name: Option<String>,
    pub pack_version: Option<String>,
    pub icon_path: Option<String>,
}

#[derive(Default)]
pub struct LinkInfo {
    pub kind: Option<InstanceKind>,
    pub source: Option<String>,
    pub pack_name: Option<String>,
    pub pack_version: Option<String>,
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn bases(home: &Path) -> Vec<(&'static str, PathBuf)> {
    let mut v: Vec<(&'static str, PathBuf)> = Vec::new();
    #[cfg(target_os = "macos")]
    {
        let app = home.join("Library/Application Support");
        v.push(("Prism Launcher", app.join("PrismLauncher/instances")));
        v.push(("PolyMC", app.join("PolyMC/instances")));
        v.push(("MultiMC", app.join("multimc/instances")));
        v.push(("Modrinth App", app.join("com.modrinth.theseus/profiles")));
        v.push(("Modrinth App", app.join("ModrinthApp/profiles")));
    }
    #[cfg(target_os = "linux")]
    {
        let share = home.join(".local/share");
        v.push(("Prism Launcher", share.join("PrismLauncher/instances")));
        v.push(("PolyMC", share.join("PolyMC/instances")));
        v.push(("MultiMC", share.join("multimc/instances")));
        v.push(("Modrinth App", share.join("ModrinthApp/profiles")));
        v.push((
            "Modrinth App",
            home.join(".config/com.modrinth.theseus/profiles"),
        ));
        let flatpak = home.join(".var/app");
        v.push((
            "Prism Launcher",
            flatpak.join(
                "org.prismlauncher.PrismLauncher/data/PrismLauncher/instances",
            ),
        ));
        v.push((
            "Modrinth App",
            flatpak.join("com.modrinth.ModrinthApp/data/ModrinthApp/profiles"),
        ));
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData/Roaming"));
        v.push(("Prism Launcher", appdata.join("PrismLauncher/instances")));
        v.push(("PolyMC", appdata.join("PolyMC/instances")));
        v.push(("MultiMC", appdata.join("MultiMC/instances")));
        v.push(("Modrinth App", appdata.join("ModrinthApp/profiles")));
        v.push((
            "Modrinth App",
            appdata.join("com.modrinth.theseus/profiles"),
        ));
    }
    v
}

pub fn detect() -> Vec<DetectedInstance> {
    let mut out: Vec<DetectedInstance> = Vec::new();
    let home = match home() {
        Some(h) => h,
        None => return out,
    };

    for (launcher, base) in bases(&home) {
        if launcher == "Modrinth App" {
            detect_modrinth(&base, &mut out);
            continue;
        }
        let entries = match std::fs::read_dir(&base) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let inst = entry.path();
            if !inst.is_dir() {
                continue;
            }
            if let Some(game_dir) = resolve_game_dir(&inst) {
                let folder = inst
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let (name, minecraft, loader, loader_version) =
                    read_meta(&inst, &folder);
                let link = read_mmc_link(&inst);
                out.push(DetectedInstance {
                    launcher: launcher.to_string(),
                    name,
                    game_dir: game_dir.to_string_lossy().to_string(),
                    minecraft,
                    loader,
                    loader_version,
                    kind: link.kind.unwrap_or(InstanceKind::Local),
                    source: link.source,
                    pack_name: link.pack_name,
                    pack_version: link.pack_version,
                    icon_path: find_instance_icon(&inst, Some(&base)),
                });
            }
        }
    }

    for dir in vanilla_dirs(&home) {
        if dir.join("launcher_profiles.json").is_file()
            || dir.join("options.txt").is_file()
        {
            let (name, minecraft, loader, loader_version) = read_vanilla(&dir);
            out.push(DetectedInstance {
                launcher: "Minecraft Launcher".into(),
                name,
                game_dir: dir.to_string_lossy().to_string(),
                minecraft,
                loader,
                loader_version,
                kind: InstanceKind::Local,
                source: None,
                pack_name: None,
                pack_version: None,
                icon_path: None,
            });
        }
    }

    out.sort_by(|a, b| a.game_dir.cmp(&b.game_dir));
    out.dedup_by(|a, b| a.game_dir == b.game_dir);
    out.sort_by(|a, b| {
        a.launcher
            .cmp(&b.launcher)
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

pub fn resolve_folder(dir: &Path) -> DetectedInstance {
    let folder = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let game_dir = resolve_game_dir(dir)
        .or_else(|| {
            [".minecraft", "minecraft"]
                .iter()
                .map(|s| dir.join(s))
                .find(|c| c.is_dir())
        })
        .unwrap_or_else(|| dir.to_path_buf());

    let is_mmc = dir.join("instance.cfg").is_file()
        || dir.join("mmc-pack.json").is_file();

    let (name, minecraft, loader, loader_version, launcher, link) = if is_mmc {
        let (name, mc, loader, lv) = read_meta(dir, &folder);
        (name, mc, loader, lv, "Prism / MultiMC", read_mmc_link(dir))
    } else if dir.join("launcher_profiles.json").is_file() {
        let (name, mc, loader, lv) = read_vanilla(dir);
        (
            name,
            mc,
            loader,
            lv,
            "Minecraft Launcher",
            LinkInfo::default(),
        )
    } else {
        let (mc, loader, lv) = read_env(&game_dir);
        (
            folder.clone(),
            mc,
            loader,
            lv,
            "Folder",
            LinkInfo::default(),
        )
    };

    let icon_path = find_instance_icon(dir, None).or_else(|| {
        let p = game_dir.join("icon.png");
        p.is_file().then(|| p.to_string_lossy().to_string())
    });

    DetectedInstance {
        launcher: launcher.into(),
        name,
        game_dir: game_dir.to_string_lossy().to_string(),
        minecraft,
        loader,
        loader_version,
        kind: link.kind.unwrap_or(InstanceKind::Local),
        source: link.source,
        pack_name: link.pack_name,
        pack_version: link.pack_version,
        icon_path,
    }
}

fn find_instance_icon(inst_root: &Path, base: Option<&Path>) -> Option<String> {
    let icon_key = std::fs::read_to_string(inst_root.join("instance.cfg"))
        .ok()
        .and_then(|cfg| {
            cfg.lines().find_map(|l| {
                l.strip_prefix("iconKey=").map(|v| v.trim().to_string())
            })
        })
        .filter(|k| !k.is_empty());
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(k) = &icon_key {
        candidates.push(inst_root.join(format!("{k}.png")));
    }
    candidates.push(inst_root.join("icon.png"));
    if let (Some(base), Some(k)) = (base, &icon_key) {
        if let Some(root) = base.parent() {
            candidates.push(root.join("icons").join(format!("{k}.png")));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().to_string())
}

fn resolve_game_dir(inst: &Path) -> Option<PathBuf> {
    let candidates = [
        inst.join(".minecraft"),
        inst.join("minecraft"),
        inst.to_path_buf(),
    ];
    for c in candidates {
        if c.join("mods").is_dir()
            || c.join("config").is_dir()
            || c.join("options.txt").is_file()
        {
            return Some(c);
        }
    }
    if inst.join("instance.cfg").is_file() {
        for sub in [".minecraft", "minecraft"] {
            let c = inst.join(sub);
            if c.is_dir() {
                return Some(c);
            }
        }
    }
    None
}

type Meta = (Option<String>, Option<String>, Option<String>);

fn read_meta(
    inst: &Path,
    folder: &str,
) -> (String, Option<String>, Option<String>, Option<String>) {
    let mut name = folder.to_string();
    if let Ok(cfg) = std::fs::read_to_string(inst.join("instance.cfg")) {
        for line in cfg.lines() {
            if let Some(value) = line.strip_prefix("name=") {
                if !value.trim().is_empty() {
                    name = value.trim().to_string();
                }
            }
        }
    }
    let (minecraft, loader, loader_version) =
        read_mmc_pack(inst).unwrap_or((None, None, None));
    (name, minecraft, loader, loader_version)
}

fn read_mmc_link(inst: &Path) -> LinkInfo {
    let cfg = match std::fs::read_to_string(inst.join("instance.cfg")) {
        Ok(c) => c,
        Err(_) => return LinkInfo::default(),
    };
    let mut managed = false;
    let mut kind = String::new();
    let mut pack_name = String::new();
    let mut pack_version = String::new();
    for line in cfg.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "ManagedPack" => managed = value.eq_ignore_ascii_case("true"),
            "ManagedPackType" => kind = value.to_string(),
            "ManagedPackName" => pack_name = value.to_string(),
            "ManagedPackVersionName" => pack_version = value.to_string(),
            _ => {}
        }
    }
    if !managed {
        return LinkInfo::default();
    }
    let source = match kind.to_lowercase().as_str() {
        "modrinth" => Some("modrinth".to_string()),
        "flame" | "curseforge" => Some("curseforge".to_string()),
        "" => None,
        other => Some(other.to_string()),
    };
    LinkInfo {
        kind: Some(InstanceKind::Modpack),
        source,
        pack_name: Some(pack_name).filter(|s| !s.is_empty()),
        pack_version: Some(pack_version).filter(|s| !s.is_empty()),
    }
}

fn modrinth_loader(raw: &str) -> Option<String> {
    match raw.to_lowercase().as_str() {
        "vanilla" => Some("vanilla".into()),
        "fabric" => Some("fabric".into()),
        "quilt" => Some("quilt".into()),
        "forge" => Some("forge".into()),
        "neoforge" | "neoforged" => Some("neoforge".into()),
        _ => None,
    }
}

fn modrinth_loader_idx(i: i64) -> Option<String> {
    match i {
        1 => Some("forge".into()),
        2 => Some("fabric".into()),
        3 => Some("quilt".into()),
        4 => Some("neoforge".into()),
        _ => None,
    }
}

fn detect_modrinth(profiles_dir: &Path, out: &mut Vec<DetectedInstance>) {
    let db = match profiles_dir.parent() {
        Some(p) => p.join("app.db"),
        None => return,
    };
    if !db.is_file() {
        return;
    }
    let conn = match rusqlite::Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(c) => c,
        Err(_) => return,
    };

    const NEW: &str = "SELECT i.path, i.name, cs.game_version, cs.loader, cs.loader_version, \
         il.link_kind, il.imported_name, il.imported_version_number \
         FROM instances i \
         LEFT JOIN instance_content_sets cs ON i.applied_content_set_id = cs.id \
         LEFT JOIN instance_links il ON il.instance_id = i.id";
    const OLD: &str =
        "SELECT path, name, game_version, mod_loader, mod_loader_version, \
         NULL, NULL, NULL FROM profiles";
    if !read_modrinth_rows(&conn, NEW, profiles_dir, out) {
        read_modrinth_rows(&conn, OLD, profiles_dir, out);
    }
}

fn modrinth_link(
    link_kind: Option<String>,
    imported_name: Option<String>,
    imported_version: Option<String>,
) -> LinkInfo {
    match link_kind.as_deref() {
        Some("imported_modpack") => LinkInfo {
            kind: Some(InstanceKind::Modpack),
            source: Some("modrinth".into()),
            pack_name: imported_name.filter(|s| !s.is_empty()),
            pack_version: imported_version.filter(|s| !s.is_empty()),
        },
        Some("server_project") => LinkInfo {
            kind: Some(InstanceKind::Server),
            source: Some("modrinth".into()),
            pack_name: imported_name.filter(|s| !s.is_empty()),
            pack_version: imported_version.filter(|s| !s.is_empty()),
        },
        _ => LinkInfo::default(),
    }
}

fn read_modrinth_rows(
    conn: &rusqlite::Connection,
    sql: &str,
    profiles_dir: &Path,
    out: &mut Vec<DetectedInstance>,
) -> bool {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, rusqlite::types::Value>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, Option<String>>(6)?,
            r.get::<_, Option<String>>(7)?,
        ))
    });
    let rows = match rows {
        Ok(r) => r,
        Err(_) => return true,
    };
    for row in rows.flatten() {
        let (
            path,
            name,
            minecraft,
            loader_val,
            loader_version,
            link_kind,
            imp_name,
            imp_ver,
        ) = row;
        let game_dir = profiles_dir.join(&path);
        if !game_dir.is_dir() {
            continue;
        }
        let loader = match loader_val {
            rusqlite::types::Value::Text(s) => modrinth_loader(&s),
            rusqlite::types::Value::Integer(i) => modrinth_loader_idx(i),
            _ => None,
        };
        let link = modrinth_link(link_kind, imp_name, imp_ver);
        let icon_path = ["icon.png", "icon.jpg"]
            .iter()
            .map(|f| game_dir.join(f))
            .find(|p| p.is_file())
            .map(|p| p.to_string_lossy().to_string());
        out.push(DetectedInstance {
            launcher: "Modrinth App".into(),
            name: name.filter(|n| !n.is_empty()).unwrap_or(path),
            game_dir: game_dir.to_string_lossy().to_string(),
            minecraft,
            loader,
            loader_version: loader_version.filter(|v| !v.is_empty()),
            kind: link.kind.unwrap_or(InstanceKind::Local),
            source: link.source,
            pack_name: link.pack_name,
            pack_version: link.pack_version,
            icon_path,
        });
    }
    true
}

fn read_mmc_pack(inst: &Path) -> Option<Meta> {
    let text = std::fs::read_to_string(inst.join("mmc-pack.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let mut minecraft = None;
    let mut loader = None;
    let mut loader_version = None;
    for comp in json.get("components").and_then(|c| c.as_array())?.iter() {
        let uid = comp.get("uid").and_then(|u| u.as_str()).unwrap_or("");
        let version = comp
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        match uid {
            "net.minecraft" => minecraft = version,
            "net.fabricmc.fabric-loader" => {
                loader = Some("fabric".into());
                loader_version = version;
            }
            "org.quiltmc.quilt-loader" => {
                loader = Some("quilt".into());
                loader_version = version;
            }
            "net.minecraftforge" => {
                loader = Some("forge".into());
                loader_version = version;
            }
            "net.neoforged" => {
                loader = Some("neoforge".into());
                loader_version = version;
            }
            _ => {}
        }
    }
    Some((minecraft, loader, loader_version))
}

fn mmc_root(game_dir: &Path) -> Option<PathBuf> {
    [
        Some(game_dir.to_path_buf()),
        game_dir.parent().map(|p| p.to_path_buf()),
    ]
    .into_iter()
    .flatten()
    .find(|d| d.join("mmc-pack.json").is_file())
}

fn modrinth_db(game_dir: &Path) -> Option<(PathBuf, String)> {
    let profiles_dir = game_dir.parent()?;
    let db = profiles_dir.parent()?.join("app.db");
    if !db.is_file() {
        return None;
    }
    let key = game_dir.file_name()?.to_string_lossy().to_string();
    Some((db, key))
}

fn vanilla_root(game_dir: &Path) -> Option<PathBuf> {
    game_dir
        .join("launcher_profiles.json")
        .is_file()
        .then(|| game_dir.to_path_buf())
}

pub fn env_writable(game_dir: &Path) -> bool {
    mmc_root(game_dir).is_some()
        || modrinth_db(game_dir).is_some()
        || vanilla_root(game_dir).is_some()
}

pub fn read_env(game_dir: &Path) -> Meta {
    if let Some(root) = mmc_root(game_dir) {
        if let Some(meta) = read_mmc_pack(&root) {
            return meta;
        }
    }
    if let Some(meta) = read_modrinth_env(game_dir) {
        return meta;
    }
    if vanilla_root(game_dir).is_some() {
        let (_, mc, loader, loader_version) = read_vanilla(game_dir);
        if mc.is_some() || loader.is_some() {
            return (mc, loader, loader_version);
        }
    }
    (None, None, None)
}

pub fn write_env(
    game_dir: &Path,
    minecraft: &str,
    loader: &str,
    loader_version: Option<&str>,
) -> Result<bool> {
    if let Some(root) = mmc_root(game_dir) {
        return write_mmc_env(&root, minecraft, loader, loader_version);
    }
    if modrinth_db(game_dir).is_some() {
        return write_modrinth_env(game_dir, minecraft, loader, loader_version);
    }
    if let Some(root) = vanilla_root(game_dir) {
        return write_vanilla_env(&root, minecraft, loader, loader_version);
    }
    Ok(false)
}

fn write_mmc_env(
    root: &Path,
    minecraft: &str,
    loader: &str,
    loader_version: Option<&str>,
) -> Result<bool> {
    let path = root.join("mmc-pack.json");
    let mut json: Value =
        serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    let comps = json
        .get_mut("components")
        .and_then(|c| c.as_array_mut())
        .ok_or_else(|| anyhow!("mmc-pack.json has no components list"))?;
    comps.retain(|c| {
        !matches!(
            c.get("uid").and_then(|u| u.as_str()).unwrap_or(""),
            "net.minecraft"
                | "net.fabricmc.intermediary"
                | "net.fabricmc.fabric-loader"
                | "org.quiltmc.quilt-loader"
                | "net.minecraftforge"
                | "net.neoforged"
        )
    });
    let mut fresh = vec![json!({
        "uid": "net.minecraft",
        "version": minecraft,
        "important": true,
    })];
    if matches!(loader, "fabric" | "quilt") {
        fresh.push(json!({
            "uid": "net.fabricmc.intermediary",
            "version": minecraft,
            "dependencyOnly": true,
        }));
    }
    if let Some(uid) = dist::loader_uid(loader) {
        fresh.push(json!({
            "uid": uid,
            "version": loader_version.unwrap_or_default(),
        }));
    }
    for (i, comp) in fresh.into_iter().enumerate() {
        comps.insert(i, comp);
    }
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&json)?),
    )?;
    Ok(true)
}

fn read_modrinth_env(game_dir: &Path) -> Option<Meta> {
    let (db, key) = modrinth_db(game_dir)?;
    let conn = rusqlite::Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;
    let (mc, loader_val, loader_version) = conn
        .query_row(
            "SELECT game_version, mod_loader, mod_loader_version FROM profiles WHERE path = ?1",
            [&key],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, rusqlite::types::Value>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .ok()?;
    let loader = match loader_val {
        rusqlite::types::Value::Text(s) => modrinth_loader(&s),
        rusqlite::types::Value::Integer(i) => modrinth_loader_idx(i),
        _ => None,
    };
    Some((mc, loader, loader_version))
}

fn write_modrinth_env(
    game_dir: &Path,
    minecraft: &str,
    loader: &str,
    loader_version: Option<&str>,
) -> Result<bool> {
    let (db, key) = match modrinth_db(game_dir) {
        Some(x) => x,
        None => return Ok(false),
    };
    let conn = rusqlite::Connection::open(&db)?;
    conn.busy_timeout(std::time::Duration::from_millis(3000))?;
    let existing = conn
        .query_row(
            "SELECT mod_loader FROM profiles WHERE path = ?1",
            [&key],
            |r| r.get::<_, rusqlite::types::Value>(0),
        )
        .unwrap_or(rusqlite::types::Value::Null);
    let loader_val = match existing {
        rusqlite::types::Value::Integer(_) => {
            rusqlite::types::Value::Integer(modrinth_loader_code(loader))
        }
        _ => rusqlite::types::Value::Text(loader.to_string()),
    };
    let updated = conn.execute(
        "UPDATE profiles SET game_version = ?1, mod_loader = ?2, mod_loader_version = ?3 WHERE path = ?4",
        rusqlite::params![minecraft, loader_val, loader_version, key],
    )?;
    Ok(updated > 0)
}

fn write_vanilla_env(
    root: &Path,
    minecraft: &str,
    loader: &str,
    loader_version: Option<&str>,
) -> Result<bool> {
    let path = root.join("launcher_profiles.json");
    let mut json: Value =
        serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    let profiles = json
        .get_mut("profiles")
        .and_then(|p| p.as_object_mut())
        .ok_or_else(|| anyhow!("launcher_profiles.json has no profiles"))?;
    let key = profiles
        .iter()
        .max_by(|a, b| {
            let la = a.1.get("lastUsed").and_then(|v| v.as_str()).unwrap_or("");
            let lb = b.1.get("lastUsed").and_then(|v| v.as_str()).unwrap_or("");
            la.cmp(lb)
        })
        .map(|(k, _)| k.clone());
    let key = match key {
        Some(k) => k,
        None => return Ok(false),
    };
    let version_id = vanilla_version_id(minecraft, loader, loader_version);
    if let Some(prof) = profiles.get_mut(&key).and_then(|p| p.as_object_mut()) {
        prof.insert("lastVersionId".into(), json!(version_id));
    }
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&json)?),
    )?;
    Ok(true)
}

fn vanilla_version_id(
    minecraft: &str,
    loader: &str,
    loader_version: Option<&str>,
) -> String {
    match (loader, loader_version) {
        ("fabric", Some(v)) => format!("fabric-loader-{v}-{minecraft}"),
        ("quilt", Some(v)) => format!("quilt-loader-{v}-{minecraft}"),
        ("forge", Some(v)) => format!("{minecraft}-forge-{v}"),
        ("neoforge", Some(v)) => format!("neoforge-{v}"),
        _ => minecraft.to_string(),
    }
}

fn modrinth_loader_code(loader: &str) -> i64 {
    match loader {
        "forge" => 1,
        "fabric" => 2,
        "quilt" => 3,
        "neoforge" => 4,
        _ => 0,
    }
}

#[allow(clippy::vec_init_then_push)]
fn vanilla_dirs(home: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "macos")]
    v.push(home.join("Library/Application Support/minecraft"));
    #[cfg(target_os = "linux")]
    v.push(home.join(".minecraft"));
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData/Roaming"));
        v.push(appdata.join(".minecraft"));
    }
    v
}

fn read_vanilla(
    game_dir: &Path,
) -> (String, Option<String>, Option<String>, Option<String>) {
    let mut name = "Minecraft".to_string();
    let mut meta: Meta = (None, None, None);
    if let Ok(text) =
        std::fs::read_to_string(game_dir.join("launcher_profiles.json"))
    {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(profiles) =
                json.get("profiles").and_then(|p| p.as_object())
            {
                let mut best_used = String::new();
                let mut best: Option<&serde_json::Value> = None;
                for prof in profiles.values() {
                    let last_used = prof
                        .get("lastUsed")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if best.is_none() || last_used > best_used.as_str() {
                        best_used = last_used.to_string();
                        best = Some(prof);
                    }
                }
                if let Some(prof) = best {
                    if let Some(n) = prof.get("name").and_then(|v| v.as_str()) {
                        if !n.trim().is_empty() {
                            name = n.trim().to_string();
                        }
                    }
                    if let Some(vid) =
                        prof.get("lastVersionId").and_then(|v| v.as_str())
                    {
                        meta = parse_version_id(vid);
                    }
                }
            }
        }
    }
    (name, meta.0, meta.1, meta.2)
}

fn parse_version_id(id: &str) -> Meta {
    if let Some(rest) = id.strip_prefix("fabric-loader-") {
        if let Some((lv, mc)) = rest.split_once('-') {
            return (
                Some(mc.to_string()),
                Some("fabric".into()),
                Some(lv.to_string()),
            );
        }
    }
    if let Some(rest) = id.strip_prefix("quilt-loader-") {
        if let Some((lv, mc)) = rest.split_once('-') {
            return (
                Some(mc.to_string()),
                Some("quilt".into()),
                Some(lv.to_string()),
            );
        }
    }
    if let Some(rest) = id.strip_prefix("neoforge-") {
        let parts: Vec<&str> = rest.split('.').collect();
        let mc = if parts.len() >= 2 {
            if parts[1] == "0" {
                Some(format!("1.{}", parts[0]))
            } else {
                Some(format!("1.{}.{}", parts[0], parts[1]))
            }
        } else {
            None
        };
        return (mc, Some("neoforge".into()), Some(rest.to_string()));
    }
    if let Some((mc, lv)) = id.split_once("-forge-") {
        return (
            Some(mc.to_string()),
            Some("forge".into()),
            Some(lv.to_string()),
        );
    }
    if id
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        return (Some(id.to_string()), None, None);
    }
    (None, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_modpack_maps_to_a_modpack_link() {
        let link = modrinth_link(
            Some("imported_modpack".into()),
            Some("Bog Mouse Land V3".into()),
            Some("1.0.0".into()),
        );
        assert_eq!(link.kind, Some(InstanceKind::Modpack));
        assert_eq!(link.source.as_deref(), Some("modrinth"));
        assert_eq!(link.pack_name.as_deref(), Some("Bog Mouse Land V3"));
        assert_eq!(link.pack_version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn server_project_maps_to_a_server_link() {
        let link = modrinth_link(Some("server_project".into()), None, None);
        assert_eq!(link.kind, Some(InstanceKind::Server));
        assert_eq!(link.source.as_deref(), Some("modrinth"));
    }

    #[test]
    fn unmanaged_has_no_link() {
        let link = modrinth_link(Some("unmanaged".into()), None, None);
        assert_eq!(link.kind, None);
        assert_eq!(link.source, None);
    }
}
