mod routes;

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::Duration,
};

use axum::{Router, routing::get};
use common::Stats;
use sysinfo::{Cpu, RefreshKind, System};
use tokio::{net::TcpListener, sync::broadcast};
use tracing::Level;

use crate::routes::{ip, run, stats, stop};

#[derive(Debug)]
struct AppState {
    client: reqwest::Client,
    stats_channel: broadcast::Sender<Stats>,
}

impl AppState {
    fn new(channel: broadcast::Sender<Stats>) -> Self {
        AppState {
            client: reqwest::Client::new(),
            stats_channel: channel,
        }
    }
}

pub static SERVER_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = std::env::var("SERVER_DIR").expect("no SERVER_DIR environment variable.");
    let path = PathBuf::from(dir);

    if !path.exists() {
        panic!("mc server dir does not exist");
    }

    path
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let (tx, _rx) = broadcast::channel::<Stats>(16);
    let app_state = Arc::new(AppState::new(tx));

    let app = Router::new()
        .route("/run", get(run))
        .route("/stop", get(stop))
        .route("/stats", get(stats))
        .route("/ip", get(ip))
        .with_state(app_state.clone());

    tokio::spawn(async move {
        let mut system = System::new_with_specifics(RefreshKind::everything().without_processes());
        // Wait a bit because CPU usage is based on diff.
        tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        // Refresh CPUs again to get actual value.
        system.refresh_cpu_usage();

        let tx = &app_state.stats_channel;

        loop {
            let stats = Stats {
                cpu_usages: system.cpus().iter().map(Cpu::cpu_usage).collect(),
                ram_free: system.free_memory(),
                ram_used: system.used_memory(),
            };

            if let Err(err) = tx.send(stats) {
                tracing::warn!("failed to broadcast stats: {err}");
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
            system.refresh_specifics(RefreshKind::everything().without_processes());
        }
    });

    dotenvy::dotenv()?;

    let port = std::env::var("RUNNER_PORT")
        .map(|p| p.parse().expect("port not int"))
        .unwrap_or(4321);
    let ip = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);

    tracing::info!("running server on :{port}");
    tracing::info!("mc server at {:?}", &*SERVER_PATH);

    let listener = TcpListener::bind(ip).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
