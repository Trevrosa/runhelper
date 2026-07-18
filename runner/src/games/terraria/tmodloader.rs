use std::{collections::HashMap, env, fs::File, io::BufReader, path::Path, process::Stdio};
#[cfg(windows)]
use std::{path::PathBuf, time::SystemTime};

use crate::{
    ServerInfo,
    games::{ARG_SEP, Mod, terraria::workshop::search},
};
use flate2::bufread::GzDecoder;
#[cfg(windows)]
use reqwest::Client;
use serde::Deserialize;
use tokio::process::Command;

pub fn command(server_path: &Path) -> Command {
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

    cmd.arg(server_path.join("start-tModLoaderServer.sh"));

    if let Ok(user_args) = env::var("GAME_ARGS") {
        cmd.args(user_args.trim().split(ARG_SEP).map(ToString::to_string));
    } else {
        let config_file = env::current_dir()
            .expect("should have permission and exist")
            .join("terrariaConfig.txt");
        if config_file.try_exists().is_ok_and(|e| e) {
            cmd.arg("-config");
            cmd.arg(config_file);
        }
    }

    cmd.stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(server_path);

    cmd
}

#[cfg(windows)]
pub async fn info(
    client: &Client,
    server_path: &Path,
    mut world_path: PathBuf,
    start_time: SystemTime,
) -> anyhow::Result<ServerInfo> {
    use crate::games::version_info;
    use anyhow::Context;

    let version = version_info(&server_path.join("tModLoader.dll"))
        .context("finding version from file")?
        .file_version;

    world_path.set_extension("twld");

    let port = super::vanilla::port();

    Ok(ServerInfo {
        port,
        start_time,
        version,
        mods: mods_from_twld(client, &world_path).await?,
    })
}

#[derive(Debug, Deserialize)]
struct Twld {
    #[serde(rename = "0header")]
    header: TwldHeader,
}

#[derive(Debug, Deserialize)]
struct TwldHeader {
    // example item:
    // "RecipeBrowser": String("0.12"),
    #[serde(rename = "generatedWithMods")]
    generated_with_mods: HashMap<String, String>,
}

async fn mods_from_twld(client: &Client, path: &Path) -> anyhow::Result<Vec<Mod>> {
    let compressed = BufReader::new(File::open(path)?);
    let decoder = GzDecoder::new(compressed);

    let twld: Twld = fastnbt::from_reader(decoder)?;
    let header_mods = twld.header.generated_with_mods;

    let mut mods = Vec::with_capacity(header_mods.len());
    for (name, version) in header_mods {
        if name == "ModLoader" {
            continue;
        }

        let r#mod = match search(client, &name).await {
            Ok(item) => Mod::Resolved {
                name: name.clone(),
                author: item.author().to_string(),
                version,
                dependency: name.contains("Lib"),
                // https://docs.tmodloader.net/docs/1.4-stable/namespace_terraria_1_1_mod_loader.html#a1c82c6b1930a8ee5c45efb091a036b06
                required: ["Both", "Client"].contains(&item.modside()),
                link: item.url(),
                icon_url: item.preview_url,
            },
            Err(err) => {
                tracing::warn!("could not resolve mod {name}: {err}");
                Mod::Unresolved {
                    filename: name,
                    version: Some(version),
                    author: None,
                    website: None,
                }
            }
        };

        mods.push(r#mod);
    }

    Ok(mods)
}
