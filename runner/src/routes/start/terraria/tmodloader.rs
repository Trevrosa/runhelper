use std::{path::Path, process::Stdio};

use tokio::process::Command;

pub fn command(server_path: &Path) -> Command {
    let exe = if cfg!(windows) {
        server_path.join("LaunchUtils/busybox64.exe")
    } else {
        "bash".into()
    };

    let mut cmd = Command::new(exe);

    if cfg!(windows) {
        cmd.arg(server_path.join("start-tModLoaderServer.sh"))
            .env("WINDOWS_MAJOR", "10")
            .env("WINDOWS_MINOR", "0");
    } else {
        cmd.arg(server_path.join("start-tModLoaderServer.sh"));
    };

    cmd.arg("-config")
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
