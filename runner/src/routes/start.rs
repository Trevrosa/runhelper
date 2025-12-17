// TODO: make the two server types better integrated

mod minecraft;
mod terraria;

use std::sync::atomic::Ordering;

use axum::extract::State;
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;

use crate::{SERVER_PATH, SERVER_TYPE, ServerType, tasks, warn_error};

use super::AppState;

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

    let run = if *SERVER_TYPE == ServerType::Minecraft {
        minecraft::run(server_path)
    } else {
        terraria::run(server_path)
    };

    let child = match run {
        Ok(child) => child,
        Err(err) => {
            state.server_starting.store(false, Ordering::Release);
            return err;
        }
    };

    let Ok(mut child) = child else {
        state.server_starting.store(false, Ordering::Release);
        tracing::error!("could not start server: {}", child.unwrap_err());
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to run server");
    };

    state.server_starting.store(false, Ordering::Release);
    state.server_running.store(true, Ordering::Release);

    let Some(pid) = child.id() else {
        warn_error!("could not get server pid");
    };

    let Some(mut stdin) = child.stdin.take() else {
        warn_error!("could not get server stdin");
    };
    let Some(stdout) = child.stdout.take() else {
        warn_error!("could not get server stdin");
    };
    let Some(stderr) = child.stderr.take() else {
        warn_error!("could not get server stdin");
    };

    if *SERVER_TYPE == ServerType::Terraria {
        if cfg!(windows) {
            tokio::spawn(tasks::child_finder(state.clone(), pid));
        } else {
            state.server_pid.store(pid, Ordering::Release);
        }

        if let Err(err) = stdin.write_u8(b'\n').await {
            tracing::warn!("failed to write to stdin: {err}");
        };
    } else {
        state.server_pid.store(pid, Ordering::Release);
    }

    tokio::spawn(tasks::console_writer(state.server_stdin.subscribe(), stdin));
    tokio::spawn(tasks::console_reader(state.console_channel.clone(), stdout));
    tokio::spawn(tasks::console_reader(state.console_channel.clone(), stderr));

    tokio::spawn(tasks::server_observer(state.clone(), child));

    tracing::info!("server started!");

    (StatusCode::OK, "ran!")
}
