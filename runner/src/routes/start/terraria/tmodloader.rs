#[cfg(windows)]
use std::time::SystemTime;
use std::{path::Path, process::Stdio};

use tokio::process::Command;

use crate::ServerInfo;

pub(super) fn command(server_path: &Path) -> Command {
    let exe = if cfg!(windows) {
        server_path.join("LaunchUtils/busybox64.exe")
    } else {
        "bash".into()
    };

    let mut cmd = Command::new(exe);

    if cfg!(windows) {
        cmd.arg("bash")
            .env("WINDOWS_MAJOR", "10")
            .env("WINDOWS_MINOR", "0");
    }

    cmd.arg(server_path.join("start-tModLoaderServer.sh"))
        .arg("-config")
        .arg(
            std::env::current_dir()
                .unwrap_or_else(|_| "./".into())
                .join("terrariaConfig.txt"),
        )
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(server_path);

    cmd
}

#[cfg(windows)]
pub(super) fn info(server_path: &Path, start_time: SystemTime) -> Result<ServerInfo, &'static str> {
    let version = version(server_path).map_err(|_| "could not get version from file")?;

    Ok(ServerInfo {
        version,
        start_time,
        mods: vec![],
    })
}

#[cfg(windows)]
fn version(server_path: &Path) -> anyhow::Result<String> {
    use win32_version_info::VersionInfo;
    Ok(VersionInfo::from_file(server_path.join("tModLoader.dll"))?.file_version)
}
