mod api;
mod authorized;
mod file_server;
mod tasks;

use std::{
    env,
    str::FromStr,
    sync::{LazyLock, atomic::AtomicBool},
};

use anyhow::Context;
use reqwest::Url;
use reqwest_websocket::Bytes;
use rocket::tokio::{
    self,
    sync::broadcast::{self},
};
use rocket::{Config, routes};

use crate::{
    api::{
        console::console, ip::ip, list::list, ping::ping, running::running, start::start,
        stats::stats, stop::stop, wake::wake,
    },
    authorized::{BASIC_TOKEN, STOP_TOKEN},
    file_server::BrServer,
    tasks::{console_helper, stats_helper},
};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(target_env = "msvc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn get_runner_addr() -> anyhow::Result<Url> {
    let addr = env::var("RUNNER_ADDR").context("runner addr not found")?;
    let port = env::var("RUNNER_PORT").unwrap_or("4321".to_string());

    Ok(Url::from_str(&format!("http://{addr}:{port}"))?)
}

pub static RUNNER_ADDR: LazyLock<Url> = LazyLock::new(|| get_runner_addr().unwrap());

trait UrlExt {
    fn join_unchecked(&self, input: &str) -> Self;
}

impl UrlExt for Url {
    fn join_unchecked(&self, input: &str) -> Self {
        self.join(input).unwrap()
    }
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    tracing_subscriber::fmt().without_time().compact().init();

    if let Err(err) = dotenvy::dotenv() {
        tracing::warn!("failed to read .env: {err}");
    }

    // initialize the LazyCells
    let _ = &*BASIC_TOKEN;
    let _ = &*STOP_TOKEN;

    let config = Config {
        port: 1234,
        ..Default::default()
    };

    let client = reqwest::Client::new();

    let (stats_tx, _rx) = broadcast::channel::<Bytes>(16);
    let (console_tx, _rx) = broadcast::channel::<String>(16);

    let state = (client.clone(), stats_tx.clone());
    tokio::spawn(stats_helper(state.0, state.1));
    let state = (client.clone(), console_tx.clone());
    tokio::spawn(console_helper(state.0, state.1));

    let _ = rocket::custom(config)
        .mount("/", BrServer::new("./static"))
        .mount(
            "/api",
            routes![ip, start, stop, ping, stats, console, wake, list, running],
        )
        .manage(client)
        .manage(stats_tx)
        .manage(console_tx)
        .manage(AtomicBool::new(false))
        .launch()
        .await;

    Ok(())
}
