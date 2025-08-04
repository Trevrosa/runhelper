use std::sync::Arc;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use tokio::sync::broadcast::Receiver;

use crate::AppState;

pub async fn console(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    let channel = state.clone().console_channel.subscribe();
    ws.on_upgrade(|socket| handle_socket(socket, channel))
}

async fn handle_socket(mut socket: WebSocket, mut channel: Receiver<String>) {
    while let Ok(line) = channel.recv().await {
        if let Err(err) = socket.send(Message::text(line)).await {
            tracing::warn!("{err}, closing socket");
            break;
        }
    }
}
