use std::sync::atomic::Ordering;

use axum::extract::State;
use reqwest::StatusCode;

use super::AppState;
use crate::{SERVER_TYPE, ServerType};

pub async fn list(State(state): AppState) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "server not on!");
    }

    // TODO: should be handled better

    if *SERVER_TYPE == ServerType::Satisfactory {
        return (StatusCode::NOT_IMPLEMENTED, "unsupported");
    }

    if let Err(err) = state.server_stdin.send("/list".to_string()) {
        tracing::info!("failed to send cmd: {err}");
    }

    (StatusCode::OK, "sent /list!")
}
