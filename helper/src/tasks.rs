use std::time::Duration;

use reqwest_websocket::{Bytes, Message, RequestBuilderExt};
use rocket::{
    futures::StreamExt,
    tokio::{self, sync::broadcast::Sender},
};

use crate::{UrlExt, RUNNER_ADDR};

const WS_TIMEOUT: Duration = Duration::from_secs(2);

/// transmits the stats from the runner to a channel.
pub async fn stats_helper(client: reqwest::Client, tx: Sender<Bytes>) {
    loop {
        let resp = client
            .get(RUNNER_ADDR.join_unchecked("stats"))
            .timeout(Duration::from_secs(4))
            .upgrade()
            .send()
            .await;
        let Ok(resp) = resp else {
            tracing::warn!("failed to send websocket request, reconnecting in {WS_TIMEOUT:?}");
            tokio::time::sleep(WS_TIMEOUT).await;
            continue;
        };

        let runner_ws = resp.into_websocket().await;
        let Ok(mut runner_ws) = runner_ws else {
            tracing::warn!("failed to upgrade to websocket, reconnecting in {WS_TIMEOUT:?}");
            tokio::time::sleep(WS_TIMEOUT).await;
            continue;
        };

        tracing::info!("connected to stats websocket");

        while let Some(message) = runner_ws.next().await {
            let message = match message {
                Ok(message) => message,
                Err(err) => {
                    tracing::warn!("websocket closed: {err}");
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

        tracing::warn!("runner websocket closed, reconnecting in {WS_TIMEOUT:?}");
        tokio::time::sleep(WS_TIMEOUT).await;
    }
}

/// transmits the stats from the runner to a channel.
pub async fn console_helper(client: reqwest::Client, tx: Sender<String>) {
    loop {
        let resp = client
            .get(RUNNER_ADDR.join_unchecked("console"))
            .timeout(Duration::from_secs(4))
            .upgrade()
            .send()
            .await;
        let Ok(resp) = resp else {
            tracing::warn!("failed to send websocket request, reconnecting in {WS_TIMEOUT:?}");
            tokio::time::sleep(WS_TIMEOUT).await;
            continue;
        };

        let runner_ws = resp.into_websocket().await;
        let Ok(mut runner_ws) = runner_ws else {
            tracing::warn!("failed to upgrade to websocket, reconnecting in {WS_TIMEOUT:?}");
            tokio::time::sleep(WS_TIMEOUT).await;
            continue;
        };

        tracing::info!("connected to console websocket");

        while let Some(message) = runner_ws.next().await {
            let message = match message {
                Ok(message) => message,
                Err(err) => {
                    tracing::warn!("websocket closed: {err}");
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

        tracing::warn!("runner websocket closed, reconnecting in {WS_TIMEOUT:?}");
        tokio::time::sleep(WS_TIMEOUT).await;
    }
}
