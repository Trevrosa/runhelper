use std::sync::Arc;

use axum::{extract::State, http::StatusCode};

use crate::AppState;

/// returns the server's ip.
pub async fn ip(State(state): State<Arc<AppState>>) -> Result<String, (StatusCode, &'static str)> {
    let client = &state.client;

    let resp = client.get("https://ipinfo.io/ip").send().await;
    let Ok(resp) = resp else {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to get ip"));
    };

    let ip = resp.text().await;
    let Ok(ip) = ip else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to parse ip api result",
        ));
    };

    Ok(ip)
}
