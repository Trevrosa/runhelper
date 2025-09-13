use std::path::{Path, PathBuf};

use anyhow::anyhow;

pub fn find_platform_args(server_path: &Path) -> anyhow::Result<PathBuf> {
    let forge_dir = server_path.join("libraries/net/minecraftforge/forge/");
    let forge_dir = forge_dir
        .read_dir()?
        .next()
        .ok_or(anyhow!("no forge dir"))??;

    let args_file = if cfg!(windows) {
        "win_args.txt"
    } else {
        // assume unix
        "unix_args.txt"
    };

    Ok(PathBuf::from("@libraries/net/minecraftforge/forge/")
        .join(forge_dir.file_name())
        .join(args_file))
}

pub fn args(server_path: &Path) -> Result<Vec<String>, &'static str> {
    let mut args = Vec::new();

    if server_path.join("user_jvm_args.txt").exists() {
        args.push("@user_jvm_args.txt".to_string());
    } else {
        tracing::warn!("could not find `user_jvm_args.txt` file");
    }

    match find_platform_args(server_path) {
        Ok(path) => args.push(path.to_string_lossy().into_owned()),
        Err(err) => {
            tracing::error!("could not find forge platform args: {err}");
            return Err("could not read forge platform args");
        }
    }

    args.push("--nogui".to_string());

    Ok(args)
}
