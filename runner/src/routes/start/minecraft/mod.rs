use axum::http::StatusCode;
use reqwest::Client;
use std::{path::Path, process::Stdio, time::SystemTime};
use tokio::process::Command;

use super::common::{GameServer, RunResult, Variant};
use crate::ServerInfo;

mod meta;
mod modrinth;

pub mod forge;
pub mod paper;

pub struct Minecraft;

#[derive(Debug, Clone, Copy)]
pub enum ServerType {
    Forge,
    Paper,
    Vanilla,
}

impl GameServer<ServerType> for Minecraft {
    fn spawn(server_path: &Path, variant: ServerType) -> RunResult {
        let args = match variant {
            ServerType::Forge => forge::args(server_path),
            ServerType::Paper => paper::args(server_path),
            ServerType::Vanilla => todo!(),
        };

        let args = match args {
            Ok(args) => args,
            Err(err) => return Err((StatusCode::INTERNAL_SERVER_ERROR, err)),
        };

        let child = Command::new("java")
            .args(args)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(server_path)
            .spawn();

        Ok(child)
    }

    async fn server_info(
        client: &Client,
        server_path: &Path,
        start_time: SystemTime,
        variant: ServerType,
    ) -> anyhow::Result<ServerInfo> {
        match variant {
            ServerType::Forge => forge::info(server_path, start_time, client).await,
            ServerType::Paper => paper::info(server_path, start_time, client).await,
            ServerType::Vanilla => todo!(),
        }
    }
}

impl Variant for ServerType {
    fn detect(server_path: &Path) -> Option<Self> {
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
