use super::common::{GameServer, RunResult, Variant};
use std::{path::Path, time::SystemTime};

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
