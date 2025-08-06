use std::sync::atomic::Ordering;

use axum::extract::{Path, State};
use reqwest::StatusCode;

use super::AppState;

/// NOT meant to be accessible publicly.
pub async fn exec(Path(cmd): Path<String>, State(state): AppState) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "server not on!");
    }

    if let Err(err) = state.server_stdin.send(cmd) {
        tracing::info!("failed to send cmd: {err}");
    }

    (StatusCode::OK, "executed command!")
}
