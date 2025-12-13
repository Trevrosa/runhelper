use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use helper::UrlExt;
use reqwest_websocket as reqwest_ws;
use reqwest_ws::{Message, RequestBuilderExt};
use tokio::signal;
use tracing::instrument;

use crate::{AppState, RUNNER_ADDR};

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
#[instrument(skip_all)]
pub async fn stats_helper(state: Arc<AppState>) {
    loop {
        let runner_ws = websocket(&state.client, RUNNER_ADDR.join_unchecked("stats")).await;
        let mut runner_ws = match runner_ws {
            Ok(ws) => ws,
            Err(err) => {
                match err {
                    reqwest_websocket::Error::Reqwest(..) => {
                        tracing::error!("failed to connect, waiting {WS_TIMEOUT:?}..")
                    }
                    _ => tracing::debug!("failed to connect, waiting {WS_TIMEOUT:?}.."),
                }
                tokio::time::sleep(WS_TIMEOUT).await;
                continue;
            }
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
                if let Err(err) = state.stats.send(bytes) {
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
#[instrument(skip_all)]
pub async fn console_helper(state: Arc<AppState>) {
    loop {
        let runner_ws = websocket(&state.client, RUNNER_ADDR.join_unchecked("console")).await;
        let mut runner_ws = match runner_ws {
            Ok(ws) => ws,
            Err(err) => {
                match err {
                    reqwest_websocket::Error::Reqwest(..) => {
                        tracing::warn!("failed to connect, waiting {WS_TIMEOUT:?}..")
                    }
                    _ => tracing::debug!("failed to connect, waiting {WS_TIMEOUT:?}.."),
                }
                tokio::time::sleep(WS_TIMEOUT).await;
                continue;
            }
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
                if let Err(err) = state.console.send(text) {
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

/// ensures graceful shutdown
#[instrument(skip_all)]
pub async fn shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("shutting down..");
}
