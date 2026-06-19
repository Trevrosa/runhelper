use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::anyhow;
use reqwest::Client;
use serde::Deserialize;

use crate::{
    ServerInfo,
    routes::start::minecraft::{ModMeta, extract_jar},
};

pub fn find_platform_args(server_path: &Path) -> anyhow::Result<PathBuf> {
    let forge_dir = server_path.join("libraries/net/minecraftforge/forge/");
    let forge_dir = forge_dir
        .read_dir()?
        .next()
        .ok_or(anyhow!("no forge dir"))??;

    let args_file = if cfg!(windows) {
        "win_args.txt"
    } else {
        // assume unix
        "unix_args.txt"
    };

    Ok(PathBuf::from("@libraries/net/minecraftforge/forge/")
        .join(forge_dir.file_name())
        .join(args_file))
}

pub fn args(server_path: &Path) -> Result<Vec<String>, &'static str> {
    let mut args = Vec::new();

    if server_path.join("user_jvm_args.txt").exists() {
        args.push("@user_jvm_args.txt".to_string());
    } else {
        tracing::warn!("could not find `user_jvm_args.txt` file");
    }

    match find_platform_args(server_path) {
        Ok(path) => args.push(path.to_string_lossy().into_owned()),
        Err(err) => {
            tracing::error!("could not find forge platform args: {err}");
            return Err("could not read forge platform args");
        }
    }

    args.push("--nogui".to_string());

    Ok(args)
}

#[derive(Deserialize)]
struct Meta {
    mods: Vec<Mod>,
}

#[derive(Deserialize)]
struct Mod {
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "displayURL")]
    display_url: Option<String>,
    authors: Option<String>,
    version: String,
}

fn get_meta(file: &Path) -> anyhow::Result<ModMeta> {
    let meta = extract_jar(file, "META-INF/mods.toml")?;
    let meta = toml::from_str::<Meta>(&meta)?
        .mods
        .pop()
        .ok_or(anyhow!("mods.toml missing metadata"))?;

    Ok(ModMeta {
        name: meta.display_name,
        version: meta.version,
        authors: meta
            .authors
            .map(|a| a.split(", ").map(ToString::to_string).collect()),
        website: meta.display_url,
    })
}

pub(super) async fn info(
    server_path: &Path,
    start_time: SystemTime,
    client: &Client,
) -> anyhow::Result<ServerInfo> {
    super::get_info(
        &server_path.join("libraries/net/minecraftforge/forge"),
        &server_path.join("mods"),
        get_meta,
        start_time,
        client,
    )
    .await
}
