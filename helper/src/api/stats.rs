use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use common::Stats;
use reqwest_websocket::Bytes;
use tokio::sync::broadcast::Receiver;

/// forward the websocket from the local runner.
pub async fn stats(ws: WebSocketUpgrade, State(state): super::AppState) -> Response {
    let channel = state.stats.subscribe();
    ws.on_upgrade(|socket| handle_socket(socket, channel))
}

async fn handle_socket(mut socket: WebSocket, mut channel: Receiver<Bytes>) {
    loop {
        while let Ok(message) = channel.recv().await {
            let Ok(stats) = bitcode::deserialize::<Stats>(&message) else {
                tracing::warn!("failed to deserialize bitcode");
                continue;
            };
            let Ok(message) = serde_json::to_string(&stats) else {
                tracing::warn!("failed to deserialize to json");
                continue;
            };

            if let Err(err) = socket.send(Message::Text(message.into())).await {
                tracing::warn!("{err}, closing socket");
                break;
            }
        }
        tracing::warn!("ws closed");
    }
}
