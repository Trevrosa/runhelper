use std::{env, fs::DirEntry, path::Path};

pub fn args(server_path: &Path) -> Result<Vec<String>, &'static str> {
    let mut args = Vec::new();

    if server_path.join("user_jvm_args.txt").exists() {
        args.push("@user_jvm_args.txt".to_string());
    } else if let Ok(jvm_args) = env::var("PAPER_ARGS") {
        let jvm_args = jvm_args.trim().split(' ').map(ToString::to_string);
        args.extend(jvm_args);
    } else {
        tracing::warn!("could not find `user_jvm_args.txt` file");
    }

    args.push("-jar".to_string());

    if server_path.join("server.jar").exists() {
        args.push("server.jar".to_string());
    } else {
        let jars: Vec<DirEntry> = server_path
            .read_dir()
            .map_err(|_| "could not read server dir")?
            .flatten() // ignore errors
            .filter(|e| e.file_type().is_ok_and(|f| f.is_file()))
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jar"))
            .collect();

        let jar = if let Some(paper) = jars
            .iter()
            .find(|e| e.file_name().to_string_lossy().starts_with("paper"))
        {
            paper.file_name()
        } else if jars.len() == 1 {
            jars[0].file_name()
        } else if !jars.is_empty() {
            tracing::warn!("found multiple jars at {server_path:?}, using the first one");
            jars[0].file_name()
        } else {
            tracing::error!("no server jar found at {server_path:?}");
            return Err("no server jar found");
        };

        args.push(jar.to_string_lossy().into_owned());
    }

    args.push("nogui".to_string());

    Ok(args)
}
