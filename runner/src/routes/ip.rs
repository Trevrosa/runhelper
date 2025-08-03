use std::sync::Arc;

use axum::{extract::State, http::StatusCode};

use crate::AppState;

/// returns the server's ip.
pub async fn ip(State(state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    let client = &state.client;

    let resp = client
        .get("https://ipinfo.io/ip")
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    resp.text()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
