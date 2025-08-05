use std::env;

use reqwest::Client;
use rocket::{State, get, http::Status};
use wake_on_lan::MagicPacket;

use crate::{RUNNER_ADDR, UrlExt};

/// wake the runner
#[get("/wake")]
pub async fn wake(client: &State<Client>) -> (Status, &'static str) {
    if client
        .get(RUNNER_ADDR.join_unchecked("ping"))
        .send()
        .await
        .is_ok()
    {
        return (Status::Ok, "already awake!");
    }

    let Ok(mac) = env::var("PHYS_ADDR") else {
        tracing::error!("no PHYS_ADDR env var.");
        return (Status::InternalServerError, "PHYS_ADDR not set");
    };

    let bytes: Vec<_> = mac.split('-').collect();
    if bytes.len() != 6 {
        tracing::error!(
            "mac address invalid, expected 6 bytes but got {}",
            bytes.len()
        );
        return (Status::InternalServerError, "PHYS_ADDR invalid");
    }

    let mut mac = [0; 6];
    for (i, hex) in bytes.iter().enumerate() {
        let Ok(byte) = u8::from_str_radix(hex, 16) else {
            tracing::error!("could not parse {hex} to a byte");
            return (Status::InternalServerError, "PHYS_ADDR invalid");
        };
        mac[i] = byte;
    }

    let magic = MagicPacket::new(&mac);

    if let Err(err) = magic.send() {
        tracing::warn!("failed to send magic packet: {err}");
        return (Status::InternalServerError, "failed to send magic packet");
    }

    (Status::Ok, "requested the server to wake up!")
}
