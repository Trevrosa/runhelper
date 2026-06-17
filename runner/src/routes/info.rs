use axum::{Json, extract::State, http::StatusCode};

use super::AppState;
use crate::ServerInfo;

pub async fn info(State(state): AppState) -> Result<Json<ServerInfo>, StatusCode> {
    let Some(info) = state.server_info.read().await.clone() else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    Ok(Json(info))
}
