mod api;
mod tasks;

use std::{
    env,
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::Context;
use axum::{Router, http::StatusCode};
use reqwest::Url;
use reqwest_websocket::Bytes;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::{
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::{DefaultOnFailure, TraceLayer},
};
use tracing::{Level, level_filters::LevelFilter};
use tracing_subscriber::{
    EnvFilter, filter::Targets, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::tasks::{console_helper, stats_helper};

#[cfg(not(windows))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(windows)]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn get_runner_addr() -> anyhow::Result<Url> {
    let addr = env::var("RUNNER_ADDR").context("runner addr not found")?;
    let port = env::var("RUNNER_PORT").unwrap_or("4321".to_string());

    Ok(Url::from_str(&format!("http://{addr}:{port}"))?)
}

pub static RUNNER_ADDR: LazyLock<Url> = LazyLock::new(|| get_runner_addr().unwrap());

struct AppState {
    client: reqwest::Client,
    stats: broadcast::Sender<Bytes>,
    console: broadcast::Sender<String>,
}

impl AppState {
    fn new(stats: broadcast::Sender<Bytes>, console: broadcast::Sender<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            stats,
            console,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(err) = dotenvy::dotenv() {
        tracing::warn!("failed to read .env: {err}");
    }

    let filter = env::var(EnvFilter::DEFAULT_ENV).map_or(LevelFilter::INFO, |e| {
        Level::from_str(&e)
            .inspect_err(|e| eprintln!("defaulted to info: {e}"))
            .unwrap_or(Level::INFO)
            .into()
    });
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().compact())
        .with(Targets::new().with_target(env!("CARGO_PKG_NAME"), filter))
        .init();

    LazyLock::force(&RUNNER_ADDR);

    let (stats_tx, _rx) = broadcast::channel::<Bytes>(16);
    let (console_tx, _rx) = broadcast::channel::<String>(16);
    let app_state = Arc::new(AppState::new(stats_tx, console_tx));

    tokio::spawn(stats_helper(app_state.clone()));
    tokio::spawn(console_helper(app_state.clone()));

    let app = Router::new()
        .fallback_service(ServeDir::new("static").precompressed_br())
        .nest("/api", api::unauthed())
        .nest("/api", api::basic_auth())
        .nest("/api", api::stop_auth())
        .with_state(app_state.clone())
        .layer(TraceLayer::new_for_http().on_failure(DefaultOnFailure::new().level(Level::DEBUG)))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(5),
        ));

    let port =
        env::var("HELPER_PORT").map_or(1234, |p| p.parse().expect("configured port is not an int"));
    let ip = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);

    tracing::info!("running server on :{port}");
    tracing::info!("runner address set at {}", *RUNNER_ADDR);

    let listener = TcpListener::bind(ip).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(tasks::shutdown())
        .await?;

    Ok(())
}
