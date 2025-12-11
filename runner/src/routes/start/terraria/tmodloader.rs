use std::path::{Path, PathBuf};

/// returns (executable, args)
pub fn command(server_path: &Path) -> (PathBuf, Vec<String>) {
    let exe = server_path.join("LaunchUtils/busybox64.exe");
    let script = server_path.join("start-tModLoaderServer.sh").to_string_lossy().to_string();
    (exe, vec!["bash".to_string(), script])
}