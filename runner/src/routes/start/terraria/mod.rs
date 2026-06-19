use axum::http::StatusCode;
use std::{path::Path, sync::Arc, time::SystemTime};
use tokio::process::Child;
use tracing::warn;

use crate::AppState;

pub mod tmodloader;

#[derive(Debug)]
pub enum ServerType {
    Vanilla,
    TModLoader,
}

impl ServerType {
    pub fn detect(server_path: &Path) -> Option<Self> {
        if server_path.join("TerrariaServer.exe").exists() {
            Some(Self::Vanilla)
        } else if server_path.join("tModLoader.dll").exists() {
            Some(Self::TModLoader)
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

    let mut cmd = match server_type {
        ServerType::TModLoader => tmodloader::command(server_path),
        ServerType::Vanilla => todo!(),
    };

    let server_path = server_path.to_owned();
    tokio::spawn(async move {
        let start_time = SystemTime::now();
        let info = match server_type {
            ServerType::TModLoader => tmodloader::info(&server_path, start_time),
            ServerType::Vanilla => todo!(),
        };
        match info {
            Ok(info) => {
                state.server_info.write().await.replace(info);
            }
            Err(err) => warn!("could not find server info: {err}"),
        }
    });

    let child = cmd.spawn();

    Ok(child)
}
