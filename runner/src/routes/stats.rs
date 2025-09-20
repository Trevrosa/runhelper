use std::{sync::Arc, time::Duration};

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use common::Stats;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use crate::AppState;

pub async fn stats(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    let channel = state.clone().stats_channel.subscribe();
    ws.on_upgrade(|socket| handle_socket(socket, channel))
}

async fn handle_socket(mut socket: WebSocket, mut channel: Receiver<Stats>) {
    loop {
        match channel.recv().await {
            Ok(ref stats) => {
                let Ok(stats) = bitcode::serialize(stats) else {
                    tracing::error!("failed to serialize stats");

                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                };

                let msg = Message::binary(stats);
                // let msg = Message::text(format!("{stats:#?}"));

                if let Err(err) = socket.send(msg).await {
                    tracing::warn!("{err}, closing websocket");
                    return;
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
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
