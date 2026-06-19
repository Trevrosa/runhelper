use anyhow::{Context, anyhow};
use axum::http::StatusCode;
use reqwest::Client;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
    process::Stdio,
    sync::Arc,
    time::SystemTime,
};
use strsim::jaro_winkler;
use tokio::process::{Child, Command};
use tracing::warn;
use zip::ZipArchive;

use crate::{
    AppState, ServerInfo,
    routes::start::{Mod, minecraft::modrinth::Project},
};

mod modrinth;

pub mod forge;
pub mod paper;

#[derive(Debug)]
pub enum ServerType {
    Forge,
    Paper,
    Vanilla,
}

impl ServerType {
    pub fn detect(server_path: &Path) -> Option<Self> {
        if server_path.join("libraries/net/minecraftforge").exists() {
            Some(Self::Forge)
        } else if server_path.join("libraries/com/velocitypowered").exists() {
            Some(Self::Paper)
        } else if server_path.join("libraries/com/mojang").exists() {
            Some(Self::Vanilla)
        } else {
            None
        }
    }
}

pub fn run(
    state: Arc<AppState>,
    server_path: &Path,
) -> Result<tokio::io::Result<Child>, (StatusCode, &'static str)> {
    let Some(server_type) = ServerType::detect(server_path) else {
        tracing::warn!("no server detected at the configured path");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "no server at the configured path!",
        ));
    };
    tracing::debug!("detected server type {server_type:?}");

    let args = match server_type {
        ServerType::Forge => forge::args(server_path),
        ServerType::Paper => paper::args(server_path),
        ServerType::Vanilla => todo!(),
    };

    let args = match args {
        Ok(args) => args,
        Err(err) => return Err((StatusCode::INTERNAL_SERVER_ERROR, err)),
    };

    let owned_path = server_path.to_owned();
    let start_time = SystemTime::now();
    tokio::spawn(async move {
        let client = &state.client;
        let server_path = owned_path;
        tracing::info!("detecting server info");
        let info = match server_type {
            ServerType::Forge => forge::info(&server_path, start_time, client).await,
            ServerType::Paper => paper::info(&server_path, start_time, client).await,
            ServerType::Vanilla => todo!(),
        };
        match info {
            Ok(info) => {
                tracing::info!("found server info");
                state.server_info.write().await.replace(info);
            }
            Err(err) => warn!("could not find server info: {err}"),
        }
    });

    let child = Command::new("java")
        .args(args)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(server_path)
        .spawn();

    Ok(child)
}

/// Extract a file from a jar
fn extract_jar(file: &Path, path: &str) -> anyhow::Result<String> {
    let file = File::open(file)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    let mut meta = String::new();
    archive.by_path(path)?.read_to_string(&mut meta)?;
    Ok(meta)
}

#[derive(Debug)]
struct ModMeta {
    name: String,
    #[allow(unused)]
    version: String,
    authors: Option<Vec<String>>,
    website: Option<String>,
}

async fn get_info(
    versions: &Path,
    mods: &Path,
    get_meta: fn(&Path) -> anyhow::Result<ModMeta>,
    start_time: SystemTime,
    client: &Client,
) -> anyhow::Result<ServerInfo> {
    let mut versions = tokio::fs::read_dir(versions)
        .await
        .context("reading versions dir")?;

    let mut version = None;

    while let Ok(Some(file)) = versions.next_entry().await {
        if let Some(ver) = file.file_name().to_string_lossy().split('-').next() {
            version = Some(ver.to_string());
            break;
        }
    }

    let mut mod_files = tokio::fs::read_dir(mods).await.context("reading mod dir")?;
    let mut paths = Vec::new();
    while let Ok(Some(file)) = mod_files.next_entry().await {
        if file.file_type().await.is_ok_and(|f| f.is_file()) {
            paths.push(file.path());
        }
    }

    let mut mods = Vec::with_capacity(paths.len());
    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy();
        let meta = get_meta(&path);

        let query = match meta {
            Ok(ref meta) => meta.name.as_ref(),
            Err(ref err) => {
                warn!("could not find mod metadata: {err}");
                name.as_ref()
            }
        };

        let proj = modrinth::Project::find(client, query).await;
        let proj = select_meta(name.into_owned(), proj, meta);
        mods.push(proj);
    }

    Ok(ServerInfo {
        start_time,
        version: version.ok_or(anyhow!("no version found"))?,
        mods,
    })
}

fn select_meta(
    filename: String,
    proj: anyhow::Result<Project>,
    meta: anyhow::Result<ModMeta>,
) -> Mod {
    match proj {
        Ok(proj) if let Ok(meta) = meta => {
            let Some(author) = meta.authors.and_then(|a| a.first().cloned()) else {
                return Mod::Unresolved {
                    filename,
                    website: meta.website,
                    author: None,
                };
            };

            // tested jaro is usually better than levenshtein etc
            let dist = jaro_winkler(&author.to_lowercase(), &proj.author.to_lowercase());
            if dist < 0.8 {
                warn!(
                    "search result not good enough ({}~{author}={dist:.2})",
                    proj.author
                );
                Mod::Unresolved {
                    filename,
                    website: meta.website,
                    author: Some(author),
                }
            } else {
                proj.into()
            }
        }
        Ok(proj) => {
            warn!("found mod, but no file meta: {proj:?}");
            Mod::Unresolved {
                filename,
                author: None,
                website: None,
            }
        }
        Err(err) if let Ok(meta) = meta => {
            warn!("could not find mod, but have file meta: {err}");
            Mod::Unresolved {
                filename,
                author: meta.authors.and_then(|a| a.first().cloned()),
                website: meta.website,
            }
        }
        Err(err) => {
            warn!("could not find mod: {err}");
            Mod::Unresolved {
                filename,
                website: None,
                author: None,
            }
        }
    }
}
