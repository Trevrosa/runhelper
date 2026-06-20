use axum::http::StatusCode;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use common::Stats;
use futures_util::SinkExt;
use reqwest_websocket::Bytes;
use tokio::sync::broadcast::Receiver;
use tracing::warn;
use yawc::{CompressionLevel, Frame, IncomingUpgrade, UpgradeFut};

use super::AppState;

/// forward the websocket from the local runner.
pub async fn stats(ws: IncomingUpgrade, State(state): AppState) -> Response {
    let channel = state.stats.subscribe();

    let options = yawc::Options::default().with_compression_level(CompressionLevel::new(4));
    let Ok((resp, fut)) = ws.upgrade(options) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create websocket",
        )
            .into_response();
    };

    tokio::spawn(async {
        if let Err(err) = handle_socket(fut, channel).await {
            warn!("ws error: {err}");
        }
    });

    resp.into_response()
}

async fn handle_socket(fut: UpgradeFut, mut channel: Receiver<Bytes>) -> yawc::Result<()> {
    let mut socket = fut.await?;

    loop {
        while let Ok(message) = channel.recv().await {
            let Ok(stats) = bitcode::deserialize::<Stats>(&message) else {
                tracing::warn!("failed to deserialize bitcode");
                continue;
            };
            let Ok(message) = serde_json::to_string(&stats) else {
                tracing::warn!("failed to serialize to json");
                continue;
            };

            if let Err(err) = socket.send(Frame::text(message)).await {
                tracing::debug!("{err}, closing socket");
                break;
            }
        }
        tracing::debug!("ws closed");
    }
}
