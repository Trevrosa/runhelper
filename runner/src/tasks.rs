use std::{
    env,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use children::get_children;
use common::Stats;
use sysinfo::{Cpu, MemoryRefreshKind, Pid, ProcessRefreshKind, RefreshKind, System};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin},
    signal,
    sync::broadcast,
};
use tracing::instrument;

use crate::AppState;

/// how many times to wait for the server to shutdown
const SERVER_SHUTDOWN_RETRIES: u32 = 3;

/// ensures graceful shutdown
#[instrument(skip_all)]
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
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("shutting down..");

    if let Err(err) = state.server_stdin.send("/stop".to_string()) {
        tracing::warn!("could not send /stop: {err}");
    }

    let mut retries = 0;
    while state.server_running.load(Ordering::Relaxed) {
        if retries == SERVER_SHUTDOWN_RETRIES {
            tracing::warn!("reached maximum retries, shutting down anyway");
            return;
        }

        tracing::debug!("waiting for server to stop");
        tokio::time::sleep(Duration::from_secs(1)).await;
        retries += 1;

        let _ = state.server_stdin.send("/stop".to_string());
    }
}

/// a background task that refreshes and broadcasts system stats.
#[instrument(skip_all)]
pub fn stats_refresher(app_state: &Arc<AppState>) {
    let mut system = System::new_with_specifics(RefreshKind::everything().without_processes());
    // Wait a bit because CPU usage is based on diff.
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
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

        // system.processes_by_exact_name(name)

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

        if tx.send(stats).is_err() {
            tracing::warn!("channel closed, quitting");
            return;
        }

        std::thread::sleep(Duration::from_secs(1));
        system.refresh_specifics(
            RefreshKind::everything()
                .without_processes()
                .without_memory(),
        );
        system.refresh_memory_specifics(MemoryRefreshKind::everything().without_swap());
    }
}

/// waits for the server ([`Child`]) to stop
#[instrument(skip_all)]
pub async fn server_observer(state: Arc<AppState>, mut child: Child) {
    if let Err(err) = child.wait().await {
        tracing::warn!("could not wait for server exit: {err}");
    }

    state.set_stopped();

    tracing::info!("server stopped");
}

#[instrument(skip_all)]
pub async fn console_writer(mut rx: broadcast::Receiver<String>, mut stdin: ChildStdin) {
    while let Ok(cmd) = rx.recv().await {
        let write_1 = stdin.write_all(cmd.as_bytes()).await;
        let write_2 = stdin.write_u8(b'\n').await;

        if let Err(err) = write_1.or(write_2) {
            tracing::warn!("could not write to stdin: {err}");
        }
    }
}

/// a background task that reads the stdout of the server (if running)
#[instrument(skip_all)]
pub async fn console_reader<C: AsyncRead + Unpin>(tx: broadcast::Sender<String>, console: C) {
    let mut console = BufReader::new(console).lines();

    let show_console = env::var("SHOW_CONSOLE").is_ok_and(|v| v == "true");
    let mut log = if show_console {
        Some(tokio::io::stdout())
    } else {
        None
    };

    while let Ok(Some(line)) = console.next_line().await {
        if let Some(ref mut log) = log {
            let _ = log.write_all(line.as_bytes()).await;
            let _ = log.write_u8(b'\n').await;
        }

        // // its from /list, safe to send raw.
        // if line.contains("]: There are") {
        //     if let Err(err) = tx.send(line) {
        //         tracing::warn!("failed to broadcast: {err}");
        //     }
        //     continue;
        // }

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
}

/// gets the real pid after it spawns
pub async fn child_finder(state: Arc<AppState>, parent: u32) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let Ok(children) = get_children(parent) else {
            tracing::warn!("failed to get server children");
            continue;
        };

        let child = children.iter().find(|e| e.name == "dotnet.exe");
        let Some(child) = child else {
            tracing::warn!("real child not spawned yet");
            continue;
        };

        tracing::info!("found real child! ({})", child.pid);

        state.server_pid.store(child.pid, Ordering::Release);
        return;
    }
}
