use std::sync::{Arc, atomic::Ordering};

use axum::extract::{Path, State};
use reqwest::StatusCode;

use crate::{AppState, routes::exec_cmd};

/// NOT meant to be accessible publicly.
pub async fn exec(
    Path(cmd): Path<String>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "server not on!");
    }

    exec_cmd(state.server_stdin.write().await, &cmd).await
}
