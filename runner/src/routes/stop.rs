use std::{
    process::{ExitStatus, Stdio},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use axum::extract::State;
use reqwest::StatusCode;
use tokio::process::Command;

use crate::AppState;

pub async fn stop(State(state): State<Arc<AppState>>) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::TOO_MANY_REQUESTS, "already stopped!");
    }

    if state.server_stopping.load(Ordering::Relaxed) {
        tracing::warn!("ignoring stop request, already stopping");
        return (StatusCode::TOO_MANY_REQUESTS, "already stopping!");
    }

    tracing::info!("received stop request");

    if let Err(err) = state.server_stdin.send("/stop".to_string()) {
        tracing::warn!("failed to send /stop: {err}");
    } else {
        state.server_stopping.store(true, Ordering::Release);
    }

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(7)).await;

        if !state.server_running.load(Ordering::Relaxed) {
            return;
        }

        tracing::info!("server still running, killing now");

        let pid = state.server_pid.load(Ordering::Relaxed);
        if pid == 0 {
            tracing::error!("server is running, but pid is 0?");
        } else {
            kill(pid).await;
        }
    });

    (StatusCode::OK, "sent /stop!")
}

/// sends `SIGKILL` on unix, `WM_QUIT` on windows.
async fn kill(pid: u32) {
    let report_status = |status: ExitStatus| {
        if !status.success() {
            tracing::error!("killing {pid} failed with {status}");
        }
    };

    if cfg!(windows) {
        match Command::new("taskkill")
            .arg("/pid")
            .arg(pid.to_string())
            .arg("/f")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(status) => report_status(status),
            Err(err) => tracing::warn!("failed to kill; {err}"),
        }
    } else if cfg!(unix) {
        match Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(status) => report_status(status),
            Err(err) => tracing::warn!("failed to kill; {err}"),
        }
    } else {
        tracing::error!("cannot kill server, not windows or unix.");
    }
}
