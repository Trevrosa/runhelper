use std::sync::{Arc, atomic::Ordering};

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::Response,
};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use crate::{AppState, helpers::CONSOLE_CHANNEL_STOP_SIGNAL};

pub async fn console(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
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
                if line == CONSOLE_CHANNEL_STOP_SIGNAL {
                    tracing::info!("server stdout channel stopped");
                    break;
                }
                if let Err(err) = socket.send(Message::text(line)).await {
                    tracing::warn!("{err}, closing socket");
                    break;
                }
            }
            Err(RecvError::Lagged(lag)) => {
                tracing::warn!("channel lagged {lag} msgs");
            }
            Err(RecvError::Closed) => {
                tracing::warn!("channel closed");
                break;
            }
        }
    }
}
