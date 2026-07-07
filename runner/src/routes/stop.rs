use std::{sync::atomic::Ordering, time::Duration};

use axum::extract::State;
use reqwest::StatusCode;
use runner::force_kill;
use tracing::warn;

use crate::games::{GameServer, Minecraft, Satisfactory, Terraria};
use crate::{SERVER_TYPE, ServerType, routes::AppState};

const WAIT_TIME: Duration = Duration::from_secs(10);
const WAIT_INCRS: Duration = Duration::from_millis(500);

pub async fn stop(State(state): AppState) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::TOO_MANY_REQUESTS, "already stopped!");
    }

    if state.server_stopping.load(Ordering::Relaxed) {
        tracing::warn!("ignoring stop request, already stopping");
        return (StatusCode::TOO_MANY_REQUESTS, "already stopping!");
    }

    tracing::info!("received stop request");

    state.server_stopping.store(true, Ordering::Release);

    let stop = match *SERVER_TYPE {
        ServerType::Minecraft => Minecraft::stop(state.clone()),
        ServerType::Terraria => Terraria::stop(state.clone()),
        ServerType::Satisfactory => Satisfactory::stop(state.clone()),
    };

    match stop {
        Ok(()) => state.server_stopping.store(true, Ordering::Release),
        Err(err) => warn!("failed to stop server: {err}"),
    }

    state.server_stopping.store(false, Ordering::Release);

    let state_1 = state.clone();
    tokio::spawn(async move {
        let loops = (WAIT_TIME.as_millis() / WAIT_INCRS.as_millis()) as usize;
        for _ in 0..loops - 2 {
            if !state_1.server_running.load(Ordering::Relaxed) {
                state_1.server_stopping.store(false, Ordering::Release);
                tracing::debug!("server stopped within {WAIT_TIME:?}!");
                break;
            }

            tokio::time::sleep(WAIT_INCRS).await;
        }
    });

    // force killer
    tokio::spawn(async move {
        tokio::time::sleep(WAIT_TIME).await;

        if !state.server_stopping.load(Ordering::Relaxed) {
            return;
        }

        if !state.server_running.load(Ordering::Relaxed) {
            return;
        }

        tracing::info!("server still running, killing now");

        let pid = state.server_pid.load(Ordering::Relaxed);
        if pid == 0 {
            tracing::error!("server is running, but pid is 0?");
        } else {
            force_kill(pid);
        }

        state.server_stopping.store(false, Ordering::Release);
    });

    (StatusCode::OK, "stopped server!")
}
