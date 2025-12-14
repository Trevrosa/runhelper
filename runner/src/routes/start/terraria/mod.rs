use axum::http::StatusCode;
use std::{path::Path, process::Stdio};
use tokio::process::{Child, Command};

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

pub fn run(server_path: &Path) -> Result<tokio::io::Result<Child>, (StatusCode, &'static str)> {
    let Some(server_type) = ServerType::detect(server_path) else {
        tracing::warn!("no server detected at the configured path");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "no server at the configured path!",
        ));
    };
    tracing::debug!("detected server type {server_type:?}");

    let cmd = match server_type {
        ServerType::TModLoader => tmodloader::command(server_path),
        ServerType::Vanilla => todo!(),
    };

    let child = Command::new(cmd.0)
        .env("WINDOWS_MAJOR", "10")
        .env("WINDOWS_MINOR", "0")
        .args(cmd.1)
        .arg("-config")
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(server_path)
        .arg(
            std::env::current_dir()
                .unwrap_or_else(|_| "./".into())
                .join("terrariaConfig.txt"),
        )
        .spawn();

    Ok(child)
}
