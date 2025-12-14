#[cfg(all(feature = "minecraft", feature = "terraria"))]
compile_error!("you can only have one of these features.");

#[cfg(feature = "minecraft")]
mod minecraft;
// TODO: what about mac/linux?
#[cfg(feature = "terraria")]
mod terraria;

use std::sync::atomic::Ordering;

use axum::extract::State;
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;

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

    #[cfg(feature = "minecraft")]
    let run = minecraft::run(server_path);
    #[cfg(feature = "terraria")]
    let run = terraria::run(server_path);

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

    // FIXME: ram/cpu stats are wrong because of pid
    let Some(parent) = child.id() else {
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

    // FIXME: dont just ignore error
    let _ = stdin.write_u8(b'\n').await;

    tokio::spawn(tasks::console_writer(state.server_stdin.subscribe(), stdin));
    tokio::spawn(tasks::console_reader(state.console_channel.clone(), stdout));
    tokio::spawn(tasks::console_reader(state.console_channel.clone(), stderr));

    tokio::spawn(tasks::server_observer(state.clone(), child));
    tokio::spawn(tasks::child_finder(state, parent));

    tracing::info!("server started!");

    (StatusCode::OK, "ran!")
}
