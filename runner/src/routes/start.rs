use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::Ordering,
};

use anyhow::anyhow;
use axum::extract::State;
use reqwest::StatusCode;
use tokio::process::Command;

use crate::{SERVER_PATH, tasks};

use super::AppState;

#[derive(Debug)]
enum ServerType {
    Forge,
    Paper,
    Vanilla,
}

impl ServerType {
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

fn find_forge_args(server_path: &Path) -> anyhow::Result<PathBuf> {
    let forge_dir = server_path.join("libraries/net/minecraftforge/forge/");
    let forge_dir = forge_dir
        .read_dir()?
        .next()
        .ok_or(anyhow!("no forge dir"))??;

    let args_file = if cfg!(windows) {
        "win_args.txt"
    } else {
        // assume unix
        "unix_args.txt"
    };

    Ok(PathBuf::from("@libraries/net/minecraftforge/forge/")
        .join(forge_dir.file_name())
        .join(args_file))
}

pub async fn start(State(state): AppState) -> (StatusCode, &'static str) {
    if state.server_running.load(Ordering::Relaxed) {
        tracing::warn!("ignoring run request, already running");
        return (StatusCode::TOO_MANY_REQUESTS, "already running..");
    }

    if state.server_starting.load(Ordering::Relaxed) {
        return (StatusCode::TOO_MANY_REQUESTS, "already starting up!");
    }

    state.server_starting.store(true, Ordering::Release);

    let server_path = SERVER_PATH.as_path();

    tracing::info!("got run request");
    let Some(server_type) = ServerType::detect(server_path) else {
        state.server_starting.store(false, Ordering::Release);
        tracing::warn!("no server detected at the configured path");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "no server at the configured path!",
        );
    };
    tracing::debug!("detected server type {server_type:?}");

    match server_type {
        ServerType::Forge => {
            let mut args = Vec::new();

            if !server_path.join("user_jvm_args.txt").exists() {
                state.server_starting.store(false, Ordering::Release);
                tracing::debug!("could not find user_jvm_args.txt file");
                return (StatusCode::INTERNAL_SERVER_ERROR, "could not read jvm args");
            }

            args.push("@user_jvm_args.txt".to_string());

            match find_forge_args(server_path) {
                Ok(path) => args.push(path.to_string_lossy().into_owned()),
                Err(err) => {
                    state.server_starting.store(false, Ordering::Release);
                    tracing::warn!("could not append forge args: {err}");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "could not read forge args",
                    );
                }
            }

            args.push("--nogui".to_string());

            let child = Command::new("java")
                .args(args)
                .stdout(Stdio::piped())
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .current_dir(server_path)
                .spawn();
            let Ok(mut child) = child else {
                state.server_starting.store(false, Ordering::Release);
                tracing::error!("could not start server: {}", child.unwrap_err());
                return (StatusCode::INTERNAL_SERVER_ERROR, "failed to run server");
            };

            state.server_starting.store(false, Ordering::Release);
            state.server_running.store(true, Ordering::Release);

            if let Some(pid) = child.id() {
                state.server_pid.store(pid, Ordering::Release);
            } else {
                tracing::warn!("could not get server pid");
            }

            if let Some(stdin) = child.stdin.take() {
                tokio::spawn(tasks::console_writer(state.server_stdin.subscribe(), stdin));
            } else {
                tracing::warn!("could not get server stdin");
            }

            if let Some(stdout) = child.stdout.take() {
                tokio::spawn(tasks::console_reader(state.console_channel.clone(), stdout));
            } else {
                tracing::warn!("could not get server stdout");
            }

            tokio::spawn(tasks::server_observer(state, child));

            tracing::info!("server started!");
        }
        ServerType::Paper | ServerType::Vanilla => {
            todo!()
        }
    }

    (StatusCode::OK, "ran!")
}
