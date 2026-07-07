use anyhow::anyhow;
use axum::http::StatusCode;
use reqwest::Client;
use std::{path::Path, process::Stdio, sync::Arc, time::SystemTime};
use tokio::process::Command;

use super::{GameServer, RunResult, Variant};
use crate::{AppState, ServerInfo};

mod meta;
mod modrinth;

mod forge;
mod paper;
mod vanilla;

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
            ServerType::Vanilla => vanilla::args(server_path),
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

    fn stop(state: Arc<AppState>) -> anyhow::Result<()> {
        if let Err(err) = state.server_stdin.send("/stop".to_string()) {
            Err(anyhow!("failed to send `/stop`: {err}"))
        } else {
            Ok(())
        }
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
            ServerType::Vanilla => vanilla::info(server_path, start_time).await,
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
