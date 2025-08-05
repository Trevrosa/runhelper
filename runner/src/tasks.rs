use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use axum::{extract::Request, middleware::Next, response::Response};
use common::Stats;
use sysinfo::{Cpu, MemoryRefreshKind, Pid, ProcessRefreshKind, RefreshKind, System};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::ChildStdout,
    signal,
};
use tracing::Level;

use crate::AppState;

/// middleware to trace a little bit
pub async fn trace(request: Request, next: Next) -> Response {
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

/// ensures graceful shutdown
pub async fn shutdown(state: Arc<AppState>) {
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
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutting down..");

    let stdin = state.server_stdin.try_write();
    if let Ok(mut stdin) = stdin {
        if let Some(stdin) = stdin.as_mut() {
            tracing::info!("sending /stop");
            stdin
                .write_all(b"/stop\n")
                .await
                .expect("could not write to server stdin");
            while state.server_running.load(Ordering::Relaxed) {
                tracing::debug!("waiting for server to stop");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    };
}

/// a background task that refreshes and broadcasts system stats.
pub async fn stats_refresher(app_state: Arc<AppState>) {
    let mut system = System::new_with_specifics(RefreshKind::everything().without_processes());
    // Wait a bit because CPU usage is based on diff.
    tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
    // Refresh CPUs again to get actual value.
    system.refresh_cpu_usage();

    let tx = &app_state.stats_channel;

    loop {
        let mut stats = Stats {
            system_cpu_usage: system.cpus().iter().map(Cpu::cpu_usage).collect(),
            system_ram_free: system.available_memory(),
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
        system.refresh_specifics(
            RefreshKind::everything()
                .without_processes()
                .without_memory(),
        );
        system.refresh_memory_specifics(MemoryRefreshKind::everything().without_swap());
    }
}

/// a background task that reads the stdout of the server (if running)
pub async fn console_reader(state: Arc<AppState>, console_stdout: ChildStdout) {
    let tx = &state.console_channel;

    let mut console = BufReader::new(console_stdout).lines();
    let mut log = tokio::io::stdout();

    while let Ok(Some(line)) = console.next_line().await {
        let _ = log.write_all(line.as_bytes()).await;
        let _ = log.write_u8(b'\n').await;

        // hide ips and coords
        let masked = line
            .chars()
            .map(|char| if char.is_ascii_digit() { '*' } else { char })
            .collect();

        if let Err(err) = tx.send(masked) {
            tracing::warn!("failed to broadcast: {err}");
        }
    }

    tracing::warn!("server stdout closed");

    state.server_pid.store(0, Ordering::Release);
    state.server_running.store(false, Ordering::Release);
}
