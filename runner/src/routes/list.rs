use std::sync::{Arc, atomic::Ordering};

use axum::extract::State;
use reqwest::StatusCode;

use crate::{AppState, routes::exec_cmd};

pub async fn list(State(state): State<Arc<AppState>>) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "server not on!");
    }

    exec_cmd(state.server_stdin.write().await, "list").await
}
