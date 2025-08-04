use std::{
    env,
    sync::atomic::{AtomicBool, Ordering},
};

use rocket::{State, get, http::Status};
use wake_on_lan::MagicPacket;

/// wake the runner
#[get("/wake")]
pub async fn wake(waking: &State<AtomicBool>) -> (Status, &'static str) {
    if waking.load(Ordering::Relaxed) {
        return (Status::TooManyRequests, "already waking");
    }

    waking.store(true, Ordering::Release);

    let Ok(mac) = env::var("PHYS_ADDR") else {
        tracing::error!("no PHYS_ADDR env var.");
        waking.store(false, Ordering::Release);
        return (Status::InternalServerError, "PHYS_ADDR not set");
    };
    let bytes: Vec<_> = mac.split('-').collect();
    if bytes.len() != 6 {
        tracing::error!(
            "mac address invalid, expected 6 bytes but got {}",
            bytes.len()
        );
        waking.store(false, Ordering::Release);
        return (Status::InternalServerError, "PHYS_ADDR invalid");
    }

    let mut mac = [0; 6];
    for (i, hex) in bytes.iter().enumerate() {
        let Ok(byte) = u8::from_str_radix(hex, 16) else {
            tracing::error!("could not parse {hex} to a byte");
            waking.store(false, Ordering::Release);
            return (Status::InternalServerError, "PHYS_ADDR invalid");
        };
        mac[i] = byte;
    }

    let magic = MagicPacket::new(&mac);

    if let Err(err) = magic.send() {
        tracing::warn!("failed to send magic packet: {err}");
        waking.store(false, Ordering::Release);
        return (Status::InternalServerError, "failed to send magic packet");
    }

    (Status::Ok, "requested the server to wake up!")
}
