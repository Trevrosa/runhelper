use std::sync::{Arc, atomic::Ordering};

use axum::extract::{Path, State};
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;

use crate::AppState;

/// NOT meant to be accessible publicly.
pub async fn exec(
    Path(cmd): Path<String>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "server not on!");
    }

    let mut stdin = state.server_stdin.write().await;

    if let Some(stdin) = stdin.as_mut() {
        let cmd = format!("/{cmd}\n");
        if let Err(err) = stdin.write_all(cmd.as_bytes()).await {
            tracing::warn!("failed to write to server stdin: {err}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to exec command.")
        } else {
            (StatusCode::OK, "executed cmd!")
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "server not on!")
    }
}
