use axum::http::StatusCode;
use std::{path::Path, process::Stdio, time::SystemTime};
use tokio::process::{Child, Command};

use crate::ServerInfo;

pub mod forge;
pub mod paper;

#[derive(Debug)]
pub enum ServerType {
    Forge,
    Paper,
    Vanilla,
}

impl ServerType {
    pub fn detect(server_path: &Path) -> Option<Self> {
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

    let args = match server_type {
        ServerType::Forge => forge::args(server_path),
        ServerType::Paper => paper::args(server_path),
        ServerType::Vanilla => todo!(),
    };

    let args = match args {
        Ok(args) => args,
        Err(err) => return Err((StatusCode::INTERNAL_SERVER_ERROR, err)),
    };

    let start_time = SystemTime::now();
    let info = match server_type {
        ServerType::Forge => forge::info(server_path, start_time),
        ServerType::Paper => paper::info(server_path, start_time),
        ServerType::Vanilla => todo!(),
    };

    let info = match info {
        Ok(info) => info,
        Err(err) => return Err((StatusCode::INTERNAL_SERVER_ERROR, err)),
    };

    let child = Command::new("java")
        .args(args)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(server_path)
        .spawn();

    Ok((child, info))
}

fn info(start_time: SystemTime, versions: &Path, mods: &Path) -> Result<ServerInfo, &'static str> {
    let Ok(mut versions) = versions.read_dir() else {
        return Err("could not read dir");
    };

    let mut version = None;

    while let Some(Ok(file)) = versions.next() {
        if let Some(ver) = file.file_name().to_string_lossy().split('-').next() {
            version = Some(ver.to_string());
            break;
        }
    }

    let mods = mods
        .read_dir()
        .map_err(|_| "could not read dir")?
        .flatten()
        .flat_map(|f| f.file_name().into_string())
        .collect();

    Ok(ServerInfo {
        start_time,
        version: version.ok_or("no version found")?,
        mods,
    })
}
