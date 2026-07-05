use anyhow::{Context, anyhow};
use reqwest::Client;
use std::{
    borrow::Cow,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    time::SystemTime,
};
use strsim::jaro_winkler;
use tracing::{debug, trace, warn};
use zip::ZipArchive;

use super::modrinth;
use crate::{Mod, ServerInfo};

/// Extract a file from a jar
pub fn extract_jar(file: &Path, path: &str) -> anyhow::Result<String> {
    let file = File::open(file)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    let mut meta = String::new();
    archive.by_path(path)?.read_to_string(&mut meta)?;
    Ok(meta)
}

#[derive(Debug, Clone)]
pub struct ModMeta {
    pub name: String,
    pub version: String,
    pub authors: Option<Vec<String>>,
    pub website: Option<String>,
}

pub async fn get_info(
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
            tracing::info!("finding mod with {query:?}");
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

fn slug_eq(slug: &str, b: &str) -> bool {
    slug.to_ascii_lowercase().replace('-', "") == b.to_ascii_lowercase().replace([' ', '-'], "")
}

const FIRST_RESULTS: usize = 7;

/// returns (`mod`, `final`). if `final` is true, stop further queries
fn select_meta(
    query: &str,
    filename: String,
    projs: anyhow::Result<Vec<modrinth::Project>>,
    meta: Result<ModMeta, Cow<'_, str>>,
) -> Option<(Mod, bool)> {
    match projs {
        Ok(projs) if let Ok(meta) = meta => {
            let Some(author) = meta.authors.and_then(|a| a.first().cloned()) else {
                return Some((
                    Mod::Unresolved {
                        filename,
                        version: Some(meta.version),
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
                    return Some(((proj, meta.version).into(), true));
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
                    return Some(((proj, meta.version).into(), true));
                }
            }

            debug!("could not find mod in first {FIRST_RESULTS} search results");
            Some((
                Mod::Unresolved {
                    filename,
                    version: Some(meta.version),
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
                    let version = filename
                        .split('-')
                        .next_back()
                        .expect("never none")
                        .split(".jar")
                        .next()
                        .expect("never none");
                    debug!("guessing version");
                    Some(((proj, version).into(), true))
                }
            } else {
                warn!("got results, but no file meta");
                Some((
                    Mod::Unresolved {
                        filename,
                        version: None,
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
                    version: Some(meta.version),
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
                    version: None,
                    website: None,
                    author: None,
                },
                false,
            ))
        }
    }
}
