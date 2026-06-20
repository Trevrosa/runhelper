use anyhow::{Context, anyhow};
use axum::http::StatusCode;
use reqwest::Client;
use std::{
    borrow::Cow,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    process::Stdio,
    sync::Arc,
    time::SystemTime,
};
use strsim::jaro_winkler;
use tokio::process::{Child, Command};
use tracing::{debug, trace, warn};
use zip::ZipArchive;

use super::{Mod, minecraft::modrinth::Project};
use crate::{AppState, ServerInfo};

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
                let elapsed = start_time.elapsed().unwrap_or_default();
                tracing::info!("found server info ({elapsed:?})");
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

#[derive(Debug, Clone)]
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
    variant: &str,
    get_meta: fn(&Path) -> Result<ModMeta, Cow<'_, str>>,
    start_time: SystemTime,
    client: &Client,
) -> anyhow::Result<ServerInfo> {
    let mut versions = tokio::fs::read_dir(versions)
        .await
        .context("reading versions dir")?;

    let mut version = None;

    while let Ok(Some(file)) = versions.next_entry().await {
        if let Some(ver) = file.file_name().to_string_lossy().split('-').next() {
            version = Some(format!("{ver} ({variant})"));
            break;
        }
    }

    let mut mod_files = tokio::fs::read_dir(mods).await.context("reading mod dir")?;
    let mut paths = Vec::new();
    while let Ok(Some(file)) = mod_files.next_entry().await {
        if file.file_type().await.is_ok_and(|f| f.is_file())
            && file.file_name().to_string_lossy().ends_with(".jar")
        {
            paths.push(file.path());
        }
    }

    let mut mods = Vec::with_capacity(paths.len());
    'paths: for path in paths {
        let name = path.file_name().unwrap().to_string_lossy();
        let meta = get_meta(&path);

        let mut queries: Vec<Cow<str>> =
            vec![name.split('-').next().unwrap_or(name.as_ref()).into()];

        match meta {
            Ok(ref meta) => queries.push(Cow::Borrowed(meta.name.as_ref())),
            Err(ref err) => warn!("could not find mod metadata: {err}"),
        }

        let mut r#mod = None;
        for query in queries.iter().rev() {
            trace!("query: {query:?}");
            let projs = modrinth::Project::find(client, query).await;
            let selected = select_meta(query, name.clone().into_owned(), projs, meta.clone());
            if let Some((this, stop)) = selected {
                r#mod = Some(this);
                if stop {
                    break;
                }
            } else {
                debug!("skipping");
                continue 'paths;
            }
        }

        if let Some(r#mod) = r#mod {
            mods.push(r#mod);
        }
    }

    Ok(ServerInfo {
        start_time,
        version: version.ok_or(anyhow!("no version found"))?,
        mods,
    })
}

fn slug_eq(a: &str, b: &str) -> bool {
    a.to_ascii_lowercase().replace('-', "") == b.to_ascii_lowercase().replace([' ', '-'], "")
}

const FIRST_RESULTS: usize = 7;

/// returns (`mod`, `final`). if `final` is true, stop further queries
fn select_meta(
    query: &str,
    filename: String,
    projs: anyhow::Result<Vec<Project>>,
    meta: Result<ModMeta, Cow<'_, str>>,
) -> Option<(Mod, bool)> {
    match projs {
        Ok(projs) if let Ok(meta) = meta => {
            let Some(author) = meta.authors.and_then(|a| a.first().cloned()) else {
                return Some((
                    Mod::Unresolved {
                        filename,
                        website: meta.website,
                        author: None,
                    },
                    false,
                ));
            };
            for proj in projs.into_iter().take(FIRST_RESULTS) {
                if slug_eq(&proj.slug, query) {
                    debug!("slug same as query");
                    if proj.client_side == "unsupported" {
                        debug!("found mod but its server-sided");
                        return None;
                    }
                    return Some((proj.into(), true));
                }

                // tested jaro is usually better than levenshtein etc
                let dist = jaro_winkler(&author.to_lowercase(), &proj.author.to_lowercase());
                if dist < 0.77 {
                    trace!("{}~{author}={dist:.2}", proj.author);
                } else if proj.client_side == "unsupported" {
                    debug!("found mod but its server-sided");
                    return None;
                } else {
                    debug!("ok");
                    return Some((proj.into(), true));
                }
            }

            debug!("could not find mod in first {FIRST_RESULTS} search results");
            Some((
                Mod::Unresolved {
                    filename,
                    website: meta.website,
                    author: Some(author),
                },
                false,
            ))
        }
        Ok(projs) => {
            if let Some(proj) = projs.into_iter().find(|proj| slug_eq(&proj.slug, query)) {
                debug!("slug same as query");
                if proj.client_side == "unsupported" {
                    debug!("found mod but its server-sided");
                    None
                } else {
                    Some((proj.into(), true))
                }
            } else {
                warn!("got results, but no file meta");
                Some((
                    Mod::Unresolved {
                        filename,
                        author: None,
                        website: None,
                    },
                    true,
                ))
            }
        }
        Err(err) if let Ok(meta) = meta => {
            warn!("could not find mod ({err}), but have file meta");
            Some((
                Mod::Unresolved {
                    filename,
                    author: meta.authors.and_then(|a| a.first().cloned()),
                    website: meta.website,
                },
                false,
            ))
        }
        Err(err) => {
            warn!("could not find mod: {err}");
            Some((
                Mod::Unresolved {
                    filename,
                    website: None,
                    author: None,
                },
                false,
            ))
        }
    }
}
