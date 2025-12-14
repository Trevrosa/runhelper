use std::{
    // process::{ExitStatus, Stdio},
    sync::atomic::Ordering,
    // time::Duration,
};

use axum::extract::State;
use kill_tree::tokio::kill_tree;
use reqwest::StatusCode;
// use tokio::process::Command;

use crate::routes::AppState;

// const WAIT_TIME: Duration = Duration::from_secs(10);
// const WAIT_INCRS: Duration = Duration::from_millis(500);

pub async fn stop(State(state): AppState) -> (StatusCode, &'static str) {
    if !state.server_running.load(Ordering::Relaxed) {
        return (StatusCode::TOO_MANY_REQUESTS, "already stopped!");
    }

    if state.server_stopping.load(Ordering::Relaxed) {
        tracing::warn!("ignoring stop request, already stopping");
        return (StatusCode::TOO_MANY_REQUESTS, "already stopping!");
    }

    tracing::info!("received stop request");

    state.server_stopping.store(true, Ordering::Release);

    let pid = state.server_pid.load(Ordering::Relaxed);
    if pid == 0 {
        tracing::error!("server is running, but pid is 0?");
    } else if let Err(err) = kill_tree(pid).await {
        state.server_stopping.store(false, Ordering::Release);

        tracing::error!("failed to kill process: {err:?}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to kill server");
    }

    state.server_stopping.store(false, Ordering::Release);

    // if let Err(err) = state.server_stdin.send("/stop".to_string()) {
    //     tracing::warn!("failed to send /stop: {err}");
    // } else {
    //     state.server_stopping.store(true, Ordering::Release);
    // }

    // let state_1 = state.clone();
    // tokio::spawn(async move {
    //     let loops = (WAIT_TIME.as_millis() / WAIT_INCRS.as_millis()) as usize;
    //     for _ in 0..loops - 2 {
    //         if !state_1.server_running.load(Ordering::Relaxed) {
    //             state_1.server_stopping.store(false, Ordering::Release);
    //             tracing::debug!("server stopped within {WAIT_TIME:?}!");
    //             break;
    //         }

    //         tokio::time::sleep(WAIT_INCRS).await;
    //     }
    // });

    // tokio::spawn(async move {
    //     tokio::time::sleep(WAIT_TIME).await;

    //     if !state.server_stopping.load(Ordering::Relaxed) {
    //         return;
    //     }

    //     if !state.server_running.load(Ordering::Relaxed) {
    //         return;
    //     }

    //     tracing::info!("server still running, killing now");

    //     let pid = state.server_pid.load(Ordering::Relaxed);
    //     if pid == 0 {
    //         tracing::error!("server is running, but pid is 0?");
    //     } else {
    //         kill(pid).await;
    //     }

    //     state.server_stopping.store(false, Ordering::Release);
    // });

    (StatusCode::OK, "stopped server!")
}

// // sends `SIGKILL` on unix, `WM_QUIT` on windows.
// async fn kill(pid: u32) {
//     let report_status = |status: ExitStatus| {
//         if !status.success() {
//             tracing::error!("killing {pid} failed with {status}");
//         }
//     };

//     if cfg!(windows) {
//         match Command::new("taskkill")
//             .arg("/pid")
//             .arg(pid.to_string())
//             .arg("/f")
//             .stdout(Stdio::null())
//             .stderr(Stdio::null())
//             .status()
//             .await
//         {
//             Ok(status) => report_status(status),
//             Err(err) => tracing::warn!("failed to kill; {err}"),
//         }
//     } else if cfg!(unix) {
//         match Command::new("kill")
//             .arg("-9")
//             .arg(pid.to_string())
//             .stdout(Stdio::null())
//             .stderr(Stdio::null())
//             .status()
//             .await
//         {
//             Ok(status) => report_status(status),
//             Err(err) => tracing::warn!("failed to kill; {err}"),
//         }
//     } else {
//         tracing::error!("cannot kill server, not windows or unix.");
//     }
// }
