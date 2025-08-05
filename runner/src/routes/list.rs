use std::sync::{Arc, atomic::Ordering};

use axum::extract::State;
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;

use crate::AppState;

pub async fn list(State(state): State<Arc<AppState>>) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "server not on!");
    }

    let mut stdin = state.server_stdin.write().await;

    if let Some(stdin) = stdin.as_mut() {
        if let Err(err) = stdin.write_all(b"/list\n").await {
            tracing::warn!("failed to write to server stdin: {err}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to exec /list.")
        } else {
            (StatusCode::OK, "executed /list!")
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "server not on!")
    }
}
