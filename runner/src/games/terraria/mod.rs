use anyhow::anyhow;
use std::{
    env::{self, current_dir},
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use super::{GameServer, RunResult, Variant};
use crate::AppState;

mod tmodloader;
mod vanilla;
mod workshop;

pub struct Terraria;

#[derive(Debug, Clone)]
pub enum ServerType {
    Vanilla,
    TModLoader(PathBuf),
}

impl GameServer<ServerType> for Terraria {
    fn spawn(server_path: &Path, variant: ServerType) -> RunResult {
        let mut cmd = match variant {
            ServerType::TModLoader(_) => tmodloader::command(server_path),
            ServerType::Vanilla => vanilla::command(server_path),
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
        client: &reqwest::Client,
        server_path: &Path,
        start_time: SystemTime,
        variant: ServerType,
    ) -> anyhow::Result<crate::ServerInfo> {
        match variant {
            ServerType::TModLoader(world) => {
                tmodloader::info(client, server_path, world, start_time).await
            }
            ServerType::Vanilla => vanilla::info(server_path, start_time),
        }
    }
}

impl Variant for ServerType {
    fn detect(server_path: &Path) -> Option<Self> {
        if server_path.join("TerrariaServer.exe").exists() {
            Some(Self::Vanilla)
        } else if server_path.join("tModLoader.dll").exists() {
            let Some(config) = find_config() else {
                tracing::error!("could not determine world location from config or args");
                return None;
            };

            let mut world = None;

            for line in config.lines() {
                if line.starts_with("world=") {
                    world = Some(line.split('=').next_back()?);
                    break;
                }
            }

            if let Some(world) = world {
                Some(Self::TModLoader(PathBuf::from(world)))
            } else {
                tracing::error!("world was not set in config");
                None
            }
        } else {
            None
        }
    }
}

fn find_config() -> Option<String> {
    let file_config = std::fs::read_to_string(current_dir().ok()?.join("terrariaConfig.txt")).ok();
    let user_config = env::var("GAME_ARGS").ok();

    file_config.or(user_config)
}
