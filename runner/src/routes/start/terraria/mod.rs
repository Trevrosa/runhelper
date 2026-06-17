use axum::http::StatusCode;
use std::{path::Path, time::SystemTime};
use tokio::process::Child;

use crate::ServerInfo;

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
    server_path: &Path,
) -> Result<(tokio::io::Result<Child>, ServerInfo), (StatusCode, &'static str)> {
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

    let start_time = SystemTime::now();
    let info = match server_type {
        ServerType::TModLoader => tmodloader::info(server_path, start_time),
        ServerType::Vanilla => todo!(),
    };

    let info = match info {
        Ok(info) => info,
        Err(err) => return Err((StatusCode::INTERNAL_SERVER_ERROR, err)),
    };

    let child = cmd.spawn();

    Ok((child, info))
}
