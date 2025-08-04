mod api;

use std::{
    env,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use anyhow::Context;
use reqwest::Url;
use reqwest_websocket::{Bytes, Message, RequestBuilderExt};
use rocket::{
    Config,
    fs::FileServer,
    futures::StreamExt,
    routes,
    tokio::{
        self,
        sync::broadcast::{self, Sender},
    },
};

use crate::api::{console::console, ip::ip, run::run, stats::stats, stop::stop, wake::wake};

fn get_runner_addr() -> anyhow::Result<Url> {
    let addr = env::var("RUNNER_ADDR").context("runner addr not found")?;
    let port = env::var("RUNNER_PORT").unwrap_or("4321".to_string());

    Ok(Url::from_str(&format!("http://{addr}:{port}"))?)
}

trait UrlExt {
    fn join_unchecked(&self, input: &str) -> Self;
}

impl UrlExt for Url {
    fn join_unchecked(&self, input: &str) -> Self {
        self.join(input).unwrap()
    }
}

const WS_TIMEOUT: Duration = Duration::from_secs(2);

/// transmits the stats from the runner to a channel.
async fn stats_helper(client: reqwest::Client, runner_addr: Arc<Url>, tx: Sender<Bytes>) {
    loop {
        let resp = client
            .get(runner_addr.join_unchecked("stats"))
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
async fn console_helper(client: reqwest::Client, runner_addr: Arc<Url>, tx: Sender<String>) {
    loop {
        let resp = client
            .get(runner_addr.join_unchecked("console"))
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

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    tracing_subscriber::fmt().compact().init();

    if let Err(err) = dotenvy::dotenv() {
        tracing::warn!("failed to read .env: {err}");
    }

    let runner_addr = match get_runner_addr() {
        Ok(addr) => addr,
        Err(err) => {
            panic!("failed to parse runner addr: {err}");
        }
    };

    let config = Config {
        port: 1234,
        ..Default::default()
    };

    let client = reqwest::Client::new();
    let runner_addr = Arc::new(runner_addr);

    let (stats_tx, _rx) = broadcast::channel::<Bytes>(16);
    let (console_tx, _rx) = broadcast::channel::<String>(16);

    let thread = (client.clone(), runner_addr.clone(), stats_tx.clone());
    tokio::spawn(stats_helper(thread.0, thread.1, thread.2));
    let thread = (client.clone(), runner_addr.clone(), console_tx.clone());
    tokio::spawn(console_helper(thread.0, thread.1, thread.2));

    rocket::custom(config)
        .mount("/", FileServer::from("./static"))
        .mount("/api", routes![ip, run, stats, stop, wake, console])
        .manage(client)
        .manage(runner_addr)
        .manage(stats_tx)
        .manage(console_tx)
        .manage(AtomicBool::new(false))
        .launch()
        .await?;

    Ok(())
}
