use std::{
    env,
    path::Path,
    process::Stdio,
    sync::{Arc, atomic::Ordering},
    time::SystemTime,
};

use anyhow::{Context, anyhow};
use reqwest::Client;
use tokio::process::Command;

use super::{GameServer, RunResult, Variant};
use crate::{
    AppState, ServerInfo,
    games::{ARG_SEP, version_info},
};

pub struct Satisfactory;

#[derive(Debug, Clone)]
pub enum ServerType {
    BaseGame,
}

impl GameServer<ServerType> for Satisfactory {
    fn spawn(server_path: &Path, _variant: ServerType) -> RunResult {
        let exe = if cfg!(windows) {
            server_path.join("Engine/Binaries/Win64/FactoryServer-Win64-Shipping-Cmd.exe")
        } else {
            "bash".into()
        };

        let mut cmd = Command::new(exe);

        if cfg!(windows) {
            use windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;

            cmd.args(["FactoryGame", "-unattended"]);
            cmd.current_dir(server_path.join("Engine/Binaries/Win64/"));
            // to prevent sending ctrlc to us (the parent process)
            cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
        } else {
            cmd.arg("FactoryServer.sh").current_dir(server_path);
        }

        if let Ok(user_args) = env::var("GAME_ARGS") {
            cmd.args(user_args.trim().split(ARG_SEP).map(ToString::to_string));
        }

        let child = cmd
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        Ok(child)
    }

    // https://satisfactory.wiki.gg/wiki/Dedicated_servers#How_do_I_gracefully_shut_down_the_Dedicated_Server?
    fn stop(state: Arc<AppState>) -> anyhow::Result<()> {
        let pid = state.server_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return Err(anyhow!("tried to stop but pid was 0"));
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent};

            let res = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) };
            if res == 0 {
                let err = std::io::Error::last_os_error();
                return Err(anyhow!("failed to send ctrlc to pid {pid}: {err}"));
            }
        }

        #[cfg(unix)]
        {
            let res = unsafe { libc::kill(pid, libc::SIGINT) };
            if res == -1 {
                let error = std::io::Error::last_os_error();
                return Err(anyhow!("failed to send SIGINT to pid {pid}: {err}"));
            }
        }

        Ok(())
    }

    async fn server_info(
        _client: &Client,
        server_path: &Path,
        start_time: SystemTime,
        _variant: ServerType,
    ) -> anyhow::Result<crate::ServerInfo> {
        let v_info = version_info(&server_path.join("FactoryServer.exe"))
            .context("getting version from file")?;
        let v_info: Vec<&str> = v_info
            .product_version // ++FactoryGame+rel-main-1.2.0-CL-495413
            .split('-')
            .skip(1)
            .collect();

        Ok(ServerInfo {
            version: format!("v{} ({}, build {})", v_info[1], v_info[0], v_info[3]),
            start_time,
            mods: vec![],
        })
    }
}

impl Variant for ServerType {
    fn detect(server_path: &Path) -> Option<Self> {
        if (cfg!(windows) && server_path.join("FactoryServer.exe").exists())
            || (cfg!(unix) && server_path.join("./FactoryServer.sh").exists())
        {
            Some(Self::BaseGame)
        } else {
            None
        }
    }
}
