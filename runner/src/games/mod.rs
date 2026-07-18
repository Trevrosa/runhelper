use std::{fmt::Debug, path::Path, str::FromStr, sync::Arc, time::SystemTime};

use reqwest::{Client, StatusCode};
use serde::Serialize;
use tokio::process::Child;
use tracing::warn;
#[cfg(windows)]
use win32_version_info::VersionInfo;

use crate::{AppState, ServerInfo};

mod minecraft;
pub use minecraft::Minecraft;
mod satisfactory;
pub use satisfactory::Satisfactory;
mod terraria;
pub use terraria::Terraria;

pub(super) const ARG_SEP: char = '\\';

pub(super) type RunResult = Result<tokio::io::Result<Child>, (StatusCode, &'static str)>;

pub(super) trait GameServer<V: Variant + Debug + Send + Clone + 'static> {
    /// Spawns the game server and sets the [`AppState`]'s `server_info` asynchronously.
    fn run(state: Arc<AppState>, server_path: &Path) -> RunResult {
        let Some(variant) = V::detect(server_path) else {
            tracing::warn!("no server detected at the configured path");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "no server at the configured path!",
            ));
        };
        tracing::debug!("detected server type {variant:?}");

        let owned_path = server_path.to_owned();
        let variant_1 = variant.clone();
        tokio::spawn(async move {
            let start_time = SystemTime::now();
            tracing::info!("detecting server info");
            match Self::server_info(&state.client, &owned_path, start_time, variant_1).await {
                Ok(info) => {
                    tracing::info!("found server info ({:?})", start_time.elapsed());
                    state.server_info.write().await.replace(info);
                }
                Err(err) => warn!("could not find server info: {err}"),
            }
        });
        Self::spawn(server_path, variant)
    }
    /// Spawns the game server.
    fn spawn(server_path: &Path, variant: V) -> RunResult;
    /// Gracefully stops the game server. Should not block.
    fn stop(state: Arc<AppState>) -> anyhow::Result<()>;
    /// Gets the server's info.
    fn server_info(
        client: &Client,
        server_path: &Path,
        start_time: SystemTime,
        variant: V,
    ) -> impl Future<Output = anyhow::Result<ServerInfo>> + Send;
}

/// A game server's variant.
pub(super) trait Variant: Sized {
    fn detect(server_path: &Path) -> Option<Self>;
}

#[derive(Debug, Clone, Serialize)]
pub enum Mod {
    Resolved {
        name: String,
        author: String,
        // this is not Optional because the filename is not shown to the user,
        // so they cannot guess the version, so this should be the correct or the best guess
        version: String,
        dependency: bool,
        required: bool,
        link: String,
        icon_url: String,
    },
    Unresolved {
        filename: String,
        // this is Optional because the user can guess the version from the filename too,
        // so this should either be correct or not there
        version: Option<String>,
        author: Option<String>,
        website: Option<String>,
    },
}

#[cfg(windows)]
fn version_info(file_path: &Path) -> anyhow::Result<VersionInfo> {
    use win32_version_info::VersionInfo;
    Ok(VersionInfo::from_file(file_path)?)
}

/// Gets `arg` from `file` first, if it's Some, else get from `GAME_ARGS`, else return the `default`.
///
/// `arg` should include all characters before the arg value, including the separator (eg. ` `, `=`)
fn get_arg_or<T: FromStr + std::fmt::Display>(arg: &str, file: Option<&Path>, default: T) -> T {
    assert!(!arg.is_empty());
    let sep = arg.chars().last().unwrap();

    let args = if let Some(file) = file
        && file.try_exists().is_ok_and(|e| e)
    {
        Some(std::fs::read_to_string(file).unwrap())
    } else if let Ok(args) = std::env::var("GAME_ARGS") {
        Some(args)
    } else {
        None
    };

    if let Some(args) = args
        && let Some(arg) = args.split(ARG_SEP).find(|a| a.starts_with(arg))
        && let Some(value) = arg.split(sep).last().and_then(|p| p.parse().ok())
    {
        value
    } else {
        tracing::warn!("using default {arg}{default}");
        default
    }
}
