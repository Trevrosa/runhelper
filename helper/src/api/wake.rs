use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use reqwest::{Client, Url};
use rocket::{State, get, http::Status, tokio};
use wake_on_lan::MagicPacket;

use crate::UrlExt;

/// wake the runner
#[get("/wake")]
pub async fn wake(
    client: &State<Client>,
    runner: &State<Arc<Url>>,
    waking: &State<AtomicBool>,
) -> Result<&'static str, Status> {
    if waking.load(Ordering::Relaxed) {
        return Err(Status::TooManyRequests);
    }

    waking.store(true, Ordering::Release);

    let Ok(mac) = env::var("PHYS_ADDR") else {
        tracing::error!("no PHYS_ADDR env var.");
        waking.store(false, Ordering::Release);
        return Err(Status::InternalServerError);
    };
    let bytes: Vec<_> = mac.split('-').collect();
    if bytes.len() != 6 {
        tracing::error!(
            "mac address invalid, expected 6 bytes but got {}",
            bytes.len()
        );
        waking.store(false, Ordering::Release);
        return Err(Status::InternalServerError);
    }

    let mut mac = [0; 6];
    for (i, hex) in bytes.iter().enumerate() {
        let Ok(byte) = u8::from_str_radix(hex, 6) else {
            tracing::error!("could not parse {hex} to a byte");
            waking.store(false, Ordering::Release);
            return Err(Status::InternalServerError);
        };
        mac[i] = byte;
    }

    let magic = MagicPacket::new(&mac);

    if let Err(err) = magic.send() {
        tracing::warn!("failed to send magic packet: {err}");
        waking.store(false, Ordering::Release);
        return Err(Status::InternalServerError);
    }

    loop {
        let resp = client.get(runner.join_unchecked("ping")).send().await;
        if resp.is_ok() {
            return Ok("woken!");
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
