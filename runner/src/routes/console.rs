use std::sync::{Arc, atomic::Ordering};

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::Response,
};
use tokio::{
    io::AsyncWriteExt,
    sync::broadcast::{Receiver, error::RecvError},
};

use crate::AppState;

pub async fn console(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    if !state.server_running.load(Ordering::Relaxed) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let channel = state.clone().console_channel.subscribe();
    Ok(ws.on_upgrade(|socket| handle_socket(socket, channel, state)))
}

async fn handle_socket(mut socket: WebSocket, mut channel: Receiver<String>, state: Arc<AppState>) {
    if state.server_ready.load(Ordering::Relaxed) {
        if let Ok(mut stdin) = state.server_stdin.try_write() {
            if let Some(stdin) = stdin.as_mut() {
                let _ = stdin.write_all(b"/list\n").await;
            }
            drop(stdin);
        }
    }

    loop {
        match channel.recv().await {
            Ok(line) => {
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
