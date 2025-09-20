use std::sync::atomic::Ordering;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::Response,
};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use crate::routes::AppState;

pub async fn console(ws: WebSocketUpgrade, State(state): AppState) -> Result<Response, StatusCode> {
    if !state.server_running.load(Ordering::Relaxed) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let channel = state.clone().console_channel.subscribe();
    Ok(ws.on_upgrade(|socket| handle_socket(socket, channel)))
}

async fn handle_socket(mut socket: WebSocket, mut channel: Receiver<String>) {
    loop {
        match channel.recv().await {
            Ok(line) => {
                if let Err(err) = socket.send(Message::text(line)).await {
                    tracing::warn!("{err}, closing socket");
                    break;
                }
            }
            Err(RecvError::Lagged(lag)) => {
                tracing::debug!("channel lagged {lag} msgs");
            }
            Err(RecvError::Closed) => {
                tracing::warn!("channel closed");
                break;
            }
        }
    }
}
