use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use axum::extract::State;
use reqwest::StatusCode;

use crate::AppState;

pub async fn stop(State(state): State<Arc<AppState>>) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::TOO_MANY_REQUESTS, "already stopped!");
    }

    if state.server_stopping.load(Ordering::Relaxed) {
        tracing::warn!("ignoring stop request, already stopping");
        return (StatusCode::TOO_MANY_REQUESTS, "already stopping!");
    }

    tracing::info!("received stop request");

    if let Err(err) = state.server_stdin.send("/stop".to_string()) {
        tracing::warn!("failed to send /stop: {err}");
    } else {
        state.server_stopping.store(true, Ordering::Release);
    }

    // sometimes the server doesnt stop from one /stop (from plugins/mods?)
    // so we wait 5 seconds and send /stop again.
    // we dont want the request to take 5 seconds though,
    // so we do the waiting in a spawned task.
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;

        if let Err(err) = state.server_stdin.send("/stop".to_string()) {
            tracing::warn!("failed to send /stop: {err}");
        }
    });

    (StatusCode::OK, "sent /stop!")
}
