use anyhow::anyhow;
use std::{path::Path, sync::Arc, time::SystemTime};

use super::{GameServer, RunResult, Variant};
use crate::AppState;

pub mod tmodloader;

pub struct Terraria;

#[derive(Debug, Clone, Copy)]
pub enum ServerType {
    Vanilla,
    TModLoader,
}

impl GameServer<ServerType> for Terraria {
    fn spawn(server_path: &Path, variant: ServerType) -> RunResult {
        let mut cmd = match variant {
            ServerType::TModLoader => tmodloader::command(server_path),
            ServerType::Vanilla => todo!(),
        };

        Ok(cmd.spawn())
    }

    fn stop(state: Arc<AppState>) -> anyhow::Result<()> {
        if let Err(err) = state.server_stdin.send("exit".to_string()) {
            Err(anyhow!("failed to send `exit`: {err}"))
        } else {
            Ok(())
        }
    }

    async fn server_info(
        _client: &reqwest::Client,
        server_path: &Path,
        start_time: SystemTime,
        variant: ServerType,
    ) -> anyhow::Result<crate::ServerInfo> {
        match variant {
            ServerType::TModLoader => tmodloader::info(server_path, start_time),
            ServerType::Vanilla => todo!(),
        }
    }
}

impl Variant for ServerType {
    fn detect(server_path: &Path) -> Option<Self> {
        if server_path.join("TerrariaServer.exe").exists() {
            Some(Self::Vanilla)
        } else if server_path.join("tModLoader.dll").exists() {
            Some(Self::TModLoader)
        } else {
            None
        }
    }
}
