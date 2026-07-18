use std::{env, process::Stdio};
#[cfg(windows)]
use std::{path::Path, time::SystemTime};

#[cfg(windows)]
use anyhow::Context;
use tokio::process::Command;

use crate::games::ARG_SEP;
#[cfg(windows)]
use crate::{ServerInfo, games::get_arg_or};

pub fn command(server_path: &Path) -> Command {
    let mut cmd = Command::new(server_path.join("TerrariaServer.exe"));

    let config_file = env::current_dir()
        .expect("should have permission and exist")
        .join("terrariaConfig.txt");

    if config_file.try_exists().is_ok_and(|e| e) {
        cmd.arg("-config");
        cmd.arg(config_file);
    } else if let Ok(user_args) = env::var("GAME_ARGS") {
        cmd.args(user_args.trim().split(ARG_SEP).map(ToString::to_string));
    }

    cmd.stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(server_path);

    cmd
}

pub fn port() -> u32 {
    get_arg_or(
        "-port ",
        Some(&env::current_dir().unwrap().join("terrariaConfig.txt")),
        7777,
    )
}

#[cfg(windows)]
pub fn info(server_path: &Path, start_time: SystemTime) -> anyhow::Result<ServerInfo> {
    use crate::games::version_info;

    let version = version_info(&server_path.join("TerrariaServer.exe"))
        .context("finding version from file")?
        .file_version;

    let port = port();

    Ok(ServerInfo {
        port,
        version,
        start_time,
        mods: vec![],
    })
}
