use std::{borrow::Cow, path::Path, time::SystemTime};

use reqwest::Client;
use serde::Deserialize;

use super::meta;
use crate::ServerInfo;
use meta::{ModMeta, extract_jar};

pub fn args(server_path: &Path) -> Result<Vec<String>, &'static str> {
    super::vanilla::args_with_jar_name(server_path, "paper")
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

pub async fn info(
    server_path: &Path,
    start_time: SystemTime,
    client: &Client,
) -> anyhow::Result<ServerInfo> {
    meta::get_info_mods(
        server_path,
        &server_path.join("versions"),
        &server_path.join("plugins"),
        "paper",
        get_meta,
        start_time,
        client,
    )
    .await
}
