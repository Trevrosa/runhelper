use std::sync::{Arc, atomic::Ordering};

use axum::extract::State;
use reqwest::StatusCode;

use crate::{routes::exec_cmd, AppState};

pub async fn stop(State(state): State<Arc<AppState>>) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::TOO_MANY_REQUESTS, "already stopped!");
    }

    if state.server_stopping.load(Ordering::Relaxed) {
        tracing::warn!("ignoring stop request, already stopping");
        return (StatusCode::TOO_MANY_REQUESTS, "already stopping!")
    }

    let stop = exec_cmd(state.server_stdin.write().await, "/stop").await;
    if stop.0.is_success() {
        state.server_stopping.store(true, Ordering::Release);
    }

    stop
}
