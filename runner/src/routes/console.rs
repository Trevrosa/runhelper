use std::{
    env,
    sync::{LazyLock, atomic::Ordering},
};

use axum::{
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::Response,
};
use serde::Deserialize;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use crate::{SERVER_TYPE, ServerType, routes::AppState};

pub static SECRET: LazyLock<Option<String>> = LazyLock::new(|| env::var("SECRET").ok());

#[derive(Deserialize)]
pub struct Secret {
    secret: Option<String>,
}

pub async fn console(
    ws: WebSocketUpgrade,
    State(state): AppState,
    secret: Query<Secret>, // safe because users only have access to `helper`, which is unable to set the query string.
) -> Result<Response, StatusCode> {
    if !state.server_running.load(Ordering::Relaxed) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let secret = secret.secret.as_ref();
    let no_filter = SECRET.as_ref().is_some_and(|s| Some(s) == secret);
    let channel = state.clone().console_channel.subscribe();
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, channel, no_filter)))
}

async fn handle_socket(mut socket: WebSocket, mut channel: Receiver<String>, no_filter: bool) {
    loop {
        match channel.recv().await {
            Ok(line) => {
                let line = if no_filter {
                    line
                } else {
                    // its from /list, safe to send raw.
                    // TODO: regex to ensure this is console output
                    if *SERVER_TYPE == ServerType::Minecraft && line.contains("]: There are") {
                        line
                    } else {
                        // hide ips and coords
                        line.chars()
                            .map(|char| if char.is_ascii_digit() { '*' } else { char })
                            .collect()
                    }
                };

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
