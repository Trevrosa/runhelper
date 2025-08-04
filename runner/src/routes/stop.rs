use std::sync::{Arc, atomic::Ordering};

use axum::extract::State;
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;

use crate::AppState;

pub async fn stop(State(state): State<Arc<AppState>>) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::NOT_MODIFIED, "already stopped");
    }

    let mut stdin = state.server_stdin.write().await;

    if let Some(stdin) = stdin.as_mut() {
        if let Err(err) = stdin.write_all(b"/stop\n").await {
            tracing::warn!("failed to write to server stdin: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to turn off server",
            )
        } else {
            (StatusCode::OK, "turned off!")
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "server not on!")
    }
}
