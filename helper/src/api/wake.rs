use std::env;

use axum::{extract::State, http::StatusCode};
use helper::UrlExt;
use wake_on_lan::MagicPacket;

use super::AppState;
use crate::RUNNER_ADDR;

/// wake the runner
pub async fn wake(State(state): AppState) -> (StatusCode, &'static str) {
    let client = &state.client;

    if client
        .get(RUNNER_ADDR.join_unchecked("ping"))
        .send()
        .await
        .is_ok()
    {
        return (StatusCode::OK, "already awake!");
    }

    let Ok(mac) = env::var("PHYS_ADDR") else {
        tracing::error!("no PHYS_ADDR env var.");
        return (StatusCode::INTERNAL_SERVER_ERROR, "PHYS_ADDR not set");
    };

    let bytes: Vec<_> = mac.split('-').collect();
    if bytes.len() != 6 {
        tracing::error!(
            "mac address invalid, expected 6 bytes but got {}",
            bytes.len()
        );
        return (StatusCode::INTERNAL_SERVER_ERROR, "PHYS_ADDR invalid");
    }

    let mut mac = [0; 6];
    for (i, hex) in bytes.iter().enumerate() {
        let Ok(byte) = u8::from_str_radix(hex, 16) else {
            tracing::error!("could not parse {hex} to a byte");
            return (StatusCode::INTERNAL_SERVER_ERROR, "PHYS_ADDR invalid");
        };
        mac[i] = byte;
    }

    let magic = MagicPacket::new(&mac);

    if let Err(err) = magic.send() {
        tracing::warn!("failed to send magic packet: {err}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to send magic packet",
        );
    }

    (StatusCode::OK, "requested the server to wake up!")
}
