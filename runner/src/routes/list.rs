use std::sync::atomic::Ordering;

use axum::extract::State;
use reqwest::StatusCode;

use super::AppState;

pub async fn list(State(state): AppState) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "server not on!");
    }

    if let Err(err) = state.server_stdin.send("/list".to_string()) {
        tracing::info!("failed to send cmd: {err}");
    } else {
        state.server_stopping.store(true, Ordering::Release);
    }

    (StatusCode::OK, "sent /list!")
}
