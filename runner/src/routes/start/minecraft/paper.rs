use std::{borrow::Cow, env, fs::DirEntry, path::Path, time::SystemTime};

use reqwest::Client;
use serde::Deserialize;

use super::{ModMeta, extract_jar};
use crate::ServerInfo;

pub(super) fn args(server_path: &Path) -> Result<Vec<String>, &'static str> {
    let mut args = Vec::new();

    if server_path.join("user_jvm_args.txt").exists() {
        args.push("@user_jvm_args.txt".to_string());
    } else if let Ok(jvm_args) = env::var("PAPER_ARGS") {
        let jvm_args = jvm_args.trim().split(' ').map(ToString::to_string);
        args.extend(jvm_args);
    } else {
        tracing::warn!("could not find `user_jvm_args.txt` file");
    }

    args.push("-jar".to_string());

    if server_path.join("server.jar").exists() {
        args.push("server.jar".to_string());
    } else {
        let jars: Vec<DirEntry> = server_path
            .read_dir()
            .map_err(|_| "could not read server dir")?
            .flatten() // ignore errors
            .filter(|e| e.file_type().is_ok_and(|f| f.is_file()))
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jar"))
            .collect();

        let jar = if let Some(paper) = jars
            .iter()
            .find(|e| e.file_name().to_string_lossy().starts_with("paper"))
        {
            paper.file_name()
        } else if jars.len() == 1 {
            jars[0].file_name()
        } else if !jars.is_empty() {
            tracing::warn!("found multiple jars at {server_path:?}, using the first one");
            jars[0].file_name()
        } else {
            tracing::error!("no server jar found at {server_path:?}");
            return Err("no server jar found");
        };

        args.push(jar.to_string_lossy().into_owned());
    }

    args.push("nogui".to_string());

    Ok(args)
}

#[derive(Deserialize)]
struct Meta {
    name: String,
    version: String,
    website: Option<String>,
    authors: Option<Vec<String>>,
    author: Option<String>,
}

fn get_meta(file: &Path) -> Result<ModMeta, Cow<'_, str>> {
    let meta = extract_jar(file, "plugin.yml").map_err(|e| e.to_string())?;
    let meta = serde_yaml::from_str::<Meta>(&meta).map_err(|e| e.to_string())?;

    let mut authors = meta.author.map(|a| vec![a]).unwrap_or_default();
    if let Some(meta_authors) = meta.authors {
        authors.extend(meta_authors);
    }

    let authors = if authors.is_empty() {
        None
    } else {
        Some(authors)
    };

    Ok(ModMeta {
        name: meta.name,
        version: meta.version,
        authors,
        website: meta.website,
    })
}

pub(super) async fn info(
    server_path: &Path,
    start_time: SystemTime,
    client: &Client,
) -> anyhow::Result<ServerInfo> {
    super::get_info(
        &server_path.join("versions"),
        &server_path.join("plugins"),
        "paper",
        get_meta,
        start_time,
        client,
    )
    .await
}
