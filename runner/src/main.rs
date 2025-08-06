mod routes;
mod tasks;

use std::{
    fmt::Debug,
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};

use axum::{Router, routing::get};
use common::Stats;
use tokio::{net::TcpListener, process::Command, sync::broadcast};
use tower_http::timeout::TimeoutLayer;
use tracing::Level;

use crate::routes::{console, exec, ip, list, ping, start, stats, stop};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(target_env = "msvc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

struct AppState {
    client: reqwest::Client,
    stats_channel: broadcast::Sender<Stats>,
    console_channel: broadcast::Sender<String>,
    /// 0 if server is not running.
    server_pid: AtomicU32,
    /// the server is starting up.
    server_starting: AtomicBool,
    /// the server is actively running.
    server_running: AtomicBool,
    /// the server is requested to be stopped
    server_stopping: AtomicBool,
    server_stdin: broadcast::Sender<String>,
}

impl Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("server_pid", &self.server_pid)
            .field("server_starting", &self.server_starting)
            .field("server_running", &self.server_running)
            .field("server_stopping", &self.server_stopping)
            .finish_non_exhaustive()
    }
}

impl AppState {
    fn new(
        stats: broadcast::Sender<Stats>,
        console: broadcast::Sender<String>,
        stdin: broadcast::Sender<String>,
    ) -> Self {
        AppState {
            client: reqwest::Client::new(),
            stats_channel: stats,
            console_channel: console,
            server_starting: AtomicBool::new(false),
            server_running: AtomicBool::new(false),
            server_stopping: AtomicBool::new(false),
            server_pid: AtomicU32::new(0),
            server_stdin: stdin,
        }
    }

    /// declare that the server is stopped.
    #[inline]
    fn set_stopped(&self) {
        self.server_pid.store(0, Ordering::Release);
        self.server_running.store(false, Ordering::Release);
        self.server_stopping.store(false, Ordering::Release);
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
    let (stdin_tx, _rx) = broadcast::channel(16);
    let app_state = Arc::new(AppState::new(stats_tx, console_tx, stdin_tx));

    let app = Router::new()
        .route("/start", get(start))
        .route("/stop", get(stop))
        .route("/ip", get(ip))
        .route("/ping", get(ping))
        .route("/list", get(list))
        .route("/exec/{*cmd}", get(exec))
        .route("/stats", get(stats))
        .route("/console", get(console))
        .with_state(app_state.clone())
        .layer(TimeoutLayer::new(Duration::from_secs(5)));

    tokio::spawn(tasks::stats_refresher(app_state.clone()));

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
    axum::serve(listener, app)
        .with_graceful_shutdown(tasks::shutdown(app_state.clone()))
        .await?;

    Ok(())
}
