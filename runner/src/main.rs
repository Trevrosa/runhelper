mod routes;

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use common::Stats;
use sysinfo::{Cpu, Pid, ProcessRefreshKind, RefreshKind, System};
use tokio::{
    io::AsyncWriteExt,
    net::TcpListener,
    process::{ChildStdin, Command},
    sync::{RwLock, broadcast},
};
use tracing::Level;

use crate::routes::{ip, ping, run, stats, stop};

#[derive(Debug)]
struct AppState {
    client: reqwest::Client,
    stats_channel: broadcast::Sender<Stats>,
    // 0 if server is not running.
    server_pid: AtomicU32,
    server_running: AtomicBool,
    server_stdin: RwLock<Option<ChildStdin>>,
}

impl AppState {
    fn new(channel: broadcast::Sender<Stats>) -> Self {
        AppState {
            client: reqwest::Client::new(),
            stats_channel: channel,
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

/// trace a little bit
async fn trace(request: Request, next: Next) -> Response {
    let span = tracing::debug_span!(
        "request",
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
    );

    let start = Instant::now();
    let resp = next.run(request).await;

    let _enter = span.enter();
    tracing::event!(Level::DEBUG, status = resp.status().as_u16(), latency = ?start.elapsed(), "finished processing request");

    resp
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--wd" {
        std::env::set_current_dir(&args[2]).expect("failed to set working dir");
    }

    let (tx, _rx) = broadcast::channel::<Stats>(16);
    let app_state = Arc::new(AppState::new(tx));

    let app = Router::new()
        .route("/run", get(run))
        .route("/start", get(run))
        .route("/stop", get(stop))
        .route("/stats", get(stats))
        .route("/ip", get(ip))
        .route("/ping", get(ping))
        .with_state(app_state.clone())
        .layer(middleware::from_fn(trace));

    let ctrlc_state = app_state.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen to ctrlc");

        tracing::info!("shutting down..");

        let mut stdin = ctrlc_state.server_stdin.write().await;
        if let Some(stdin) = stdin.as_mut() {
            stdin
                .write_all(b"/stop\n")
                .await
                .expect("could not write to server stdin");
        }

        std::process::exit(0);
    });

    tokio::spawn(async move {
        let mut system = System::new_with_specifics(RefreshKind::everything().without_processes());
        // Wait a bit because CPU usage is based on diff.
        tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        // Refresh CPUs again to get actual value.
        system.refresh_cpu_usage();

        let tx = &app_state.stats_channel;

        loop {
            let mut stats = Stats {
                system_cpu_usage: system.cpus().iter().map(Cpu::cpu_usage).collect(),
                system_ram_free: system.free_memory(),
                system_ram_used: system.used_memory(),
                server_ram_usage: None,
                server_cpu_usage: None,
                server_disk_usage: None,
            };

            let pid = app_state.server_pid.load(Ordering::Relaxed);
            if pid != 0 {
                let pid = Pid::from_u32(pid);

                system.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&[pid]),
                    true,
                    ProcessRefreshKind::everything(),
                );

                if let Some(process) = system.process(pid) {
                    stats.server_ram_usage = Some(process.memory());
                    stats.server_cpu_usage = Some(process.cpu_usage());
                    let disk = process.disk_usage();
                    stats.server_disk_usage = Some(disk.read_bytes + disk.written_bytes);
                }
            }

            if let Err(err) = tx.send(stats) {
                tracing::warn!("failed to broadcast stats: {err}");
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
            system.refresh_specifics(RefreshKind::everything().without_processes());
        }
    });

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
