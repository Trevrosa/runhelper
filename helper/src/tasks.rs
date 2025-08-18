use std::time::Duration;

use reqwest_websocket as reqwest_ws;
use reqwest_ws::{Bytes, Message, RequestBuilderExt};
use rocket::{
    futures::StreamExt,
    tokio::{self, sync::broadcast::Sender},
};

use crate::{RUNNER_ADDR, UrlExt};

pub async fn websocket(
    client: &reqwest::Client,
    url: impl reqwest::IntoUrl,
) -> Result<reqwest_ws::WebSocket, reqwest_ws::Error> {
    client
        .get(url)
        .timeout(Duration::from_secs(4))
        .upgrade()
        .send()
        .await?
        .into_websocket()
        .await
}

const WS_TIMEOUT: Duration = Duration::from_secs(2);

/// transmits the stats from the runner to a channel.
pub async fn stats_helper(client: reqwest::Client, tx: Sender<Bytes>) {
    loop {
        let runner_ws = websocket(&client, RUNNER_ADDR.join_unchecked("stats")).await;
        let Ok(mut runner_ws) = runner_ws else {
            tracing::error!("failed to connect to stats, waiting {WS_TIMEOUT:?}..");
            tokio::time::sleep(WS_TIMEOUT).await;
            continue;
        };

        tracing::info!("connected to stats");

        while let Some(message) = runner_ws.next().await {
            let message = match message {
                Ok(message) => message,
                Err(err) => {
                    tracing::warn!("stats ws closed: {err}");
                    break;
                }
            };

            if let Message::Binary(bytes) = message {
                if let Err(err) = tx.send(bytes) {
                    tracing::warn!("failed to broadcast: {err}");
                }
            } else {
                tracing::warn!("expected binary, got something else");
            }
        }

        tracing::warn!("stats ws closed, waiting {WS_TIMEOUT:?}..");
        tokio::time::sleep(WS_TIMEOUT).await;
    }
}

/// transmits the stats from the runner to a channel.
pub async fn console_helper(client: reqwest::Client, tx: Sender<String>) {
    loop {
        let runner_ws = websocket(&client, RUNNER_ADDR.join_unchecked("console")).await;
        let Ok(mut runner_ws) = runner_ws else {
            tracing::error!("failed to connect to console, waiting {WS_TIMEOUT:?}..");
            tokio::time::sleep(WS_TIMEOUT).await;
            continue;
        };

        tracing::info!("connected to console");

        while let Some(message) = runner_ws.next().await {
            let message = match message {
                Ok(message) => message,
                Err(err) => {
                    tracing::warn!("console ws closed: {err}");
                    break;
                }
            };

            if let Message::Text(text) = message {
                if let Err(err) = tx.send(text) {
                    tracing::warn!("failed to broadcast: {err}");
                }
            } else {
                tracing::warn!("expected text, got something else");
            }
        }

        tracing::warn!("console ws closed, waiting {WS_TIMEOUT:?}..");
        tokio::time::sleep(WS_TIMEOUT).await;
    }
}
