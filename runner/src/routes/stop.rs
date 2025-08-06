use std::sync::{Arc, atomic::Ordering};

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

    (StatusCode::OK, "sent /stop!")
}
