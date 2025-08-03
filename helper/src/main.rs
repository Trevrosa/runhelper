mod api;

use std::{
    env,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
};

use anyhow::Context;
use reqwest::Url;
use rocket::{Config, fs::FileServer, launch, routes};

use crate::api::{ip::ip, run::run, stats::stats, stop::stop, wake::wake};

fn get_runner_addr() -> anyhow::Result<Url> {
    let addr = env::var("RUNNER_ADDR").context("runner addr not found")?;
    let port = env::var("RUNNER_PORT").unwrap_or("4321".to_string());

    Ok(Url::from_str(&format!("http://{addr}:{port}"))?)
}

trait UrlExt {
    fn join_unchecked(&self, input: &str) -> Self;
}

impl UrlExt for Url {
    fn join_unchecked(&self, input: &str) -> Self {
        self.join(input).unwrap()
    }
}

#[launch]
fn rocket() -> _ {
    tracing_subscriber::fmt().compact().init();

    if let Err(err) = dotenvy::dotenv() {
        tracing::warn!("failed to read .env: {err}");
    }

    let runner_addr = match get_runner_addr() {
        Ok(addr) => addr,
        Err(err) => {
            panic!("failed to parse runner addr: {err}");
        }
    };

    let config = Config {
        port: 1234,
        ..Default::default()
    };

    rocket::custom(config)
        .mount("/", FileServer::from("./static"))
        .mount("/api", routes![ip, run, stats, stop, wake])
        .manage(reqwest::Client::new())
        .manage(Arc::new(runner_addr))
        .manage(AtomicBool::new(false))
}
