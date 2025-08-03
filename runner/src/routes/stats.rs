use std::{sync::Arc, time::Duration};

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use common::Stats;
use tokio::sync::broadcast::Receiver;

use crate::AppState;

pub async fn stats(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    let channel = state.clone().stats_channel.subscribe();
    ws.on_upgrade(|socket| handle_socket(socket, channel))
}

async fn handle_socket(mut socket: WebSocket, mut channel: Receiver<Stats>) {
    while let Ok(stats) = channel.recv().await {
        let msg = Message::binary(bitcode::encode(&stats));
        // let msg = Message::text(format!("{stats:#?}"));

        if let Err(err) = socket.send(msg).await {
            tracing::warn!("failed to send msg: {err}, closing websocket");
            return;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
