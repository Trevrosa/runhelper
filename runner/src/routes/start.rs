mod mc;
// TODO: what about mac/linux?
mod terraria;

use std::{process::Stdio, sync::atomic::Ordering};

use axum::extract::State;
use reqwest::StatusCode;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{SERVER_PATH, tasks, warn_error};

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
    let Some(server_type) = terraria::ServerType::detect(server_path) else {
        state.server_starting.store(false, Ordering::Release);
        tracing::warn!("no server detected at the configured path");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "no server at the configured path!",
        );
    };
    tracing::debug!("detected server type {server_type:?}");

    let cmd = match server_type {
        terraria::ServerType::TModLoader => terraria::tmodloader::command(server_path),
        _ => todo!()
    };

    let child = Command::new(cmd.0)
        .env("WINDOWS_MAJOR", "10")
        .env("WINDOWS_MINOR", "0")
        .args(cmd.1)
        .arg("-config")
        // FIXME: dont just unwrap
        .arg(std::env::current_dir().unwrap().join("terrariaConfig.txt"))
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(server_path)
        .spawn();

    // let Some(server_type) = mc::ServerType::detect(server_path) else {
    //     state.server_starting.store(false, Ordering::Release);
    //     tracing::warn!("no server detected at the configured path");
    //     return (
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         "no server at the configured path!",
    //     );
    // };
    // tracing::debug!("detected server type {server_type:?}");

    // let args = match server_type {
    //     mc::ServerType::Forge => mc::forge::args(server_path),
    //     mc::ServerType::Paper => mc::paper::args(server_path),
    //     mc::ServerType::Vanilla => todo!(),
    // };

    // let args = match args {
    //     Ok(args) => args,
    //     Err(err) => {
    //         state.server_starting.store(false, Ordering::Release);
    //         return (StatusCode::INTERNAL_SERVER_ERROR, err);
    //     }
    // };

    // let child = Command::new("java")
    //     .args(args)
    //     .stdout(Stdio::piped())
    //     .stdin(Stdio::piped())
    //     .stderr(Stdio::piped())
    //     .current_dir(server_path)
    //     .spawn();

    let Ok(mut child) = child else {
        state.server_starting.store(false, Ordering::Release);
        tracing::error!("could not start server: {}", child.unwrap_err());
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to run server");
    };

    state.server_starting.store(false, Ordering::Release);
    state.server_running.store(true, Ordering::Release);

    // FIXME: ram/cpu stats are wrong because of pid
    let Some(pid) = child.id() else {
        warn_error!("could not get server pid");
    };

    state.server_pid.store(pid, Ordering::Release);

    let Some(mut stdin) = child.stdin.take() else {
        warn_error!("could not get server stdin");
    };
    let Some(stdout) = child.stdout.take() else {
        warn_error!("could not get server stdin");
    };
    let Some(stderr) = child.stderr.take() else {
        warn_error!("could not get server stdin");
    };

    // FIXME: dont just ignore error
    let _ = stdin.write_u8(b'\n').await;

    tokio::spawn(tasks::console_writer(state.server_stdin.subscribe(), stdin));
    tokio::spawn(tasks::console_reader(state.console_channel.clone(), stdout));
    tokio::spawn(tasks::console_reader(state.console_channel.clone(), stderr));

    tokio::spawn(tasks::server_observer(state, child));

    tracing::info!("server started!");

    (StatusCode::OK, "ran!")
}
