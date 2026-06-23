// use rocket::{State, futures::SinkExt, get, tokio::sync::broadcast};
// use rocket_ws::{Channel, WebSocket};

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use tokio::sync::broadcast::Receiver;

use super::AppState;

/// forward the websocket from the local runner.
pub async fn console(ws: WebSocketUpgrade, State(state): AppState) -> Response {
    let channel = state.console.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, channel))
}

async fn handle_socket(mut socket: WebSocket, mut channel: Receiver<String>) {
    while let Ok(message) = channel.recv().await {
        if let Err(err) = socket.send(Message::Text(message.into())).await {
            tracing::debug!("{err}, closing socket");
            break;
        }
    }
    tracing::debug!("ws closed");
}
