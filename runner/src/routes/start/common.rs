use std::{fmt::Debug, path::Path, sync::Arc, time::SystemTime};

use reqwest::{Client, StatusCode};
use serde::Serialize;
use tokio::process::Child;
use tracing::warn;

use crate::{AppState, ServerInfo};

pub type RunResult = Result<tokio::io::Result<Child>, (StatusCode, &'static str)>;

pub trait GameServer<V: Variant + Debug + Send + Copy + 'static> {
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
        tokio::spawn(async move {
            let start_time = SystemTime::now();
            tracing::info!("detecting server info");
            match Self::server_info(&state.client, &owned_path, start_time, variant).await {
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
    /// Gets the server's info.
    fn server_info(
        client: &Client,
        server_path: &Path,
        start_time: SystemTime,
        variant: V,
    ) -> impl Future<Output = anyhow::Result<ServerInfo>> + Send;
}

/// A game server's variant.
pub trait Variant: Sized {
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
