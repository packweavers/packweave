use anyhow::Result;
use serde_json::json;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::instance;
use crate::lockfile::{self, target_dir};
use crate::manifest::{self, Manifest};

pub(crate) const INSTALLER: &[u8] =
    include_bytes!("../../installer/packweaver.jar");
pub(crate) const INSTALLER_JAR: &str = "packweaver.jar";

pub fn export_dist(
    pack_dir: &Path,
    out_dir: &Path,
    pack_url: &str,
    loader_version: Option<&str>,
) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;
    let mut manifest = manifest::read(pack_dir)?;
    if manifest.loader_version.as_deref().unwrap_or("").is_empty() {
        manifest.loader_version = loader_version.map(str::to_string);
    }
    let url = pack_url.trim_end_matches('/').to_string();
    let icon = icon_bytes(pack_dir);
    let instance_zip =
        out_dir.join(format!("{}.zip", sanitize(&manifest.name)));
    write_prism_instance(&manifest, &url, &instance_zip, icon.as_deref())?;
    Ok(())
}

fn icon_bytes(pack_dir: &Path) -> Option<Vec<u8>> {
    std::fs::read(instance::overrides_dir(pack_dir).join("icon.png")).ok()
}

fn write_prism_instance(
    manifest: &Manifest,
    url: &str,
    out_zip: &Path,
    icon: Option<&[u8]>,
) -> Result<()> {
    let file = std::fs::File::create(out_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);

    let key = sanitize(&manifest.name);
    let mut cfg = format!(
        "InstanceType=OneSix\nname={}\nOverrideCommands=true\nPreLaunchCommand=java -jar \"$INST_MC_DIR/{}\" \"$INST_MC_DIR\"\n",
        manifest.name, INSTALLER_JAR
    );
    if icon.is_some() {
        cfg.push_str(&format!("iconKey={key}\n"));
    }
    zip.start_file("instance.cfg", options)?;
    zip.write_all(cfg.as_bytes())?;

    if let Some(bytes) = icon {
        zip.start_file(format!("{key}.png"), options)?;
        zip.write_all(bytes)?;
    }

    zip.start_file("mmc-pack.json", options)?;
    zip.write_all(mmc_pack(manifest)?.as_bytes())?;

    zip.start_file(".minecraft/modpack.url", options)?;
    zip.write_all(format!("{url}\n").as_bytes())?;

    zip.start_file(format!(".minecraft/{INSTALLER_JAR}"), options)?;
    zip.write_all(INSTALLER)?;

    zip.finish()?;
    Ok(())
}

pub async fn export_instance(
    pack_dir: &Path,
    out_zip: &Path,
    env: &str,
    loader_version: Option<&str>,
) -> Result<()> {
    let mut manifest = manifest::read(pack_dir)?;
    let mut lock = lockfile::read(pack_dir)?;
    lock.mods.retain(|m| !m.disabled);
    if manifest.loader_version.as_deref().unwrap_or("").is_empty() {
        manifest.loader_version = loader_version.map(str::to_string);
    }
    let client = reqwest::Client::builder()
        .user_agent(
            "packweave/0.1.0 (modpack builder; https://github.com/packweave)",
        )
        .build()?;

    let file = std::fs::File::create(out_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);

    let icon = icon_bytes(pack_dir);
    let key = sanitize(&manifest.name);
    let mut cfg = format!("InstanceType=OneSix\nname={}\n", manifest.name);
    if icon.is_some() {
        cfg.push_str(&format!("iconKey={key}\n"));
    }
    zip.start_file("instance.cfg", options)?;
    zip.write_all(cfg.as_bytes())?;

    if let Some(bytes) = &icon {
        zip.start_file(format!("{key}.png"), options)?;
        zip.write_all(bytes)?;
    }

    zip.start_file("mmc-pack.json", options)?;
    zip.write_all(mmc_pack(&manifest)?.as_bytes())?;

    for m in &lock.mods {
        if env == "server" && m.server_side == "unsupported" {
            continue;
        }
        if env == "client" && m.client_side == "unsupported" {
            continue;
        }
        let src = match m.active() {
            Some(s) if !s.download_url.is_empty() => s,
            _ => continue,
        };
        let bytes = client.get(&src.download_url).send().await?.bytes().await?;
        zip.start_file(
            format!(
                ".minecraft/{}/{}",
                target_dir(m.project_type),
                src.filename
            ),
            options,
        )?;
        zip.write_all(&bytes)?;
    }

    let overrides = instance::overrides_dir(pack_dir);
    if overrides.is_dir() {
        zip_overrides(&mut zip, &overrides, &overrides, options)?;
    }

    zip.finish()?;
    Ok(())
}

fn zip_overrides(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    options: SimpleFileOptions,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)?.flatten() {
        let p = entry.path();
        if p.is_dir() {
            zip_overrides(zip, base, &p, options)?;
        } else if p.is_file() {
            let rel = p
                .strip_prefix(base)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            let bytes = std::fs::read(&p)?;
            zip.start_file(format!(".minecraft/{rel}"), options)?;
            zip.write_all(&bytes)?;
        }
    }
    Ok(())
}

pub(crate) fn mmc_pack(manifest: &Manifest) -> Result<String> {
    let mut components = vec![json!({
        "uid": "net.minecraft",
        "version": manifest.minecraft,
        "important": true,
    })];
    if matches!(manifest.loader.as_str(), "fabric" | "quilt") {
        components.push(json!({
            "uid": "net.fabricmc.intermediary",
            "version": manifest.minecraft,
            "dependencyOnly": true,
        }));
    }
    if let Some(uid) = loader_uid(&manifest.loader) {
        components.push(json!({
            "uid": uid,
            "version": manifest.loader_version.clone().unwrap_or_default(),
        }));
    }
    Ok(serde_json::to_string_pretty(&json!({
        "components": components,
        "formatVersion": 1,
    }))?)
}

pub(crate) fn loader_uid(loader: &str) -> Option<&'static str> {
    match loader {
        "fabric" => Some("net.fabricmc.fabric-loader"),
        "quilt" => Some("org.quiltmc.quilt-loader"),
        "forge" => Some("net.minecraftforge"),
        "neoforge" => Some("net.neoforged"),
        _ => None,
    }
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if s.is_empty() {
        "modpack".into()
    } else {
        s
    }
}
