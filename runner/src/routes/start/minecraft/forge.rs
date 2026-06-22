use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::anyhow;
use reqwest::Client;
use serde::Deserialize;

use super::{ModMeta, extract_jar};
use crate::ServerInfo;

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
#[serde(untagged)]
enum StringOrMore {
    Str(String),
    Vec(Vec<String>),
}

impl StringOrMore {
    fn get_all(me: Option<Self>) -> Option<Vec<String>> {
        match me? {
            Self::Str(str) => Some(str.split(", ").map(ToString::to_string).collect()),
            Self::Vec(vec) if !vec.is_empty() => Some(vec),
            Self::Vec(_) => None,
        }
    }
}

#[derive(Deserialize)]
struct Meta {
    authors: Option<StringOrMore>,
    mods: Vec<Mod>,
}

#[derive(Deserialize)]
struct Mod {
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "displayURL")]
    display_url: Option<String>,
    authors: Option<StringOrMore>,
    version: String,
}

fn get_meta(file: &Path) -> Result<ModMeta, Cow<'_, str>> {
    let meta = extract_jar(file, "META-INF/mods.toml").map_err(|e| e.to_string())?;
    let mut meta = toml::from_str::<Meta>(&meta).map_err(|e| e.to_string())?;
    let mut r#mod = meta.mods.pop().ok_or("mods.toml missing metadata")?;

    if r#mod.version == "${file.jarVersion}" {
        let manifest = extract_jar(file, "META-INF/MANIFEST.MF").map_err(|e| e.to_string())?;
        let impl_ver = manifest
            .lines()
            .find(|l| l.starts_with("Implementation-Version:"))
            .and_then(|l| l.split(": ").last())
            .ok_or("invalid MANIFEST.MF")?;
        r#mod.version = impl_ver.to_string();
    }

    let authors = StringOrMore::get_all(r#mod.authors.or(meta.authors));

    Ok(ModMeta {
        name: r#mod.display_name,
        version: r#mod.version,
        authors,
        website: r#mod.display_url,
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
        "forge",
        get_meta,
        start_time,
        client,
    )
    .await
}
