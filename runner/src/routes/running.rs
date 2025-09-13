use std::sync::atomic::Ordering;

use axum::extract::State;

use super::AppState;

/// returns the server running state
pub async fn running(State(state): AppState) -> String {
    state.server_running.load(Ordering::Relaxed).to_string()
}
