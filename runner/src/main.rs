mod helpers;
mod routes;

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, AtomicU32},
    },
};

use axum::{
    Router,
    middleware::{self},
    routing::get,
};
use common::Stats;
use tokio::{
    net::TcpListener,
    process::{ChildStdin, Command},
    sync::{RwLock, broadcast},
};
use tracing::Level;

use crate::routes::{console, exec, ip, ping, start, stats, stop};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(target_env = "msvc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Debug)]
struct AppState {
    client: reqwest::Client,
    stats_channel: broadcast::Sender<Stats>,
    console_channel: broadcast::Sender<String>,
    // 0 if server is not running.
    server_pid: AtomicU32,
    server_running: AtomicBool,
    server_stdin: RwLock<Option<ChildStdin>>,
}

impl AppState {
    fn new(stats: broadcast::Sender<Stats>, console: broadcast::Sender<String>) -> Self {
        AppState {
            client: reqwest::Client::new(),
            stats_channel: stats,
            console_channel: console,
            server_running: AtomicBool::new(false),
            server_pid: AtomicU32::new(0),
            server_stdin: RwLock::new(None),
        }
    }
}

pub static SERVER_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = std::env::var("SERVER_DIR").expect("no SERVER_DIR environment variable.");
    let path = PathBuf::from(dir);

    assert!(path.exists(), "mc server dir does not exist");

    path
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--wd" {
        std::env::set_current_dir(&args[2]).expect("failed to set working dir");
    }

    let (stats_tx, _rx) = broadcast::channel(16);
    let (console_tx, _rx) = broadcast::channel(16);
    let app_state = Arc::new(AppState::new(stats_tx, console_tx));

    let app = Router::new()
        .route("/start", get(start))
        .route("/stop", get(stop))
        .route("/ip", get(ip))
        .route("/ping", get(ping))
        .route("/exec/{*cmd}", get(exec))
        .route("/stats", get(stats))
        .route("/console", get(console))
        .with_state(app_state.clone())
        .layer(middleware::from_fn(helpers::trace));

    tokio::spawn(helpers::shutdown(app_state.clone()));
    tokio::spawn(helpers::stats_refresher(app_state.clone()));

    if let Err(err) = dotenvy::dotenv() {
        tracing::warn!("could not load .env: {err}");
    }

    let port = std::env::var("RUNNER_PORT")
        .map(|p| p.parse().expect("configured port is not an int"))
        .unwrap_or(4321);
    let ip = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);

    let server_path = &*SERVER_PATH;

    tracing::info!("running server on :{port}");
    tracing::info!("mc server set at {server_path:?}");
    tracing::info!("");
    let java_version = Command::new("java")
        .arg("--version")
        .output()
        .await
        .expect("`java --version` failed to run");
    let java_version = str::from_utf8(&java_version.stdout).expect("stdout not utf8");
    for line in java_version.split('\n') {
        if !line.is_empty() {
            tracing::info!("{line}");
        }
    }

    let listener = TcpListener::bind(ip).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
