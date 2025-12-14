use std::{env, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{Response, StatusCode},
    routing::get,
};
use tower_http::auth::AsyncRequireAuthorizationLayer;

#[macro_use]
mod make_forward;

type AppState = axum::extract::State<Arc<crate::AppState>>;

// basic
make_forward!(start, "/start");

// basic
make_forward!(ip, "/ip");

// stop
make_forward!(stop, "/stop");

make_forward!(running, "/running");

make_forward!(ping, "/ping");

make_forward!(list, "/list");

pub mod stats;

pub mod console;

// stop
pub mod wake;

pub fn unauthed() -> Router<Arc<crate::AppState>> {
    Router::new()
        .route("/stats", get(stats::stats))
        .route("/console", get(console::console))
        .route("/running", get(running::running))
        .route("/ping", get(ping::ping))
        .route("/list", get(list::list))
}

macro_rules! require_auth {
    ($token:expr) => {
        AsyncRequireAuthorizationLayer::new(|req: Request| async move {
            let unauth = Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::empty())
                .expect("should be able to create response");

            let Some(token) = req.headers().get("token") else {
                return Err(unauth);
            };

            if token == $token {
                Ok(req)
            } else {
                Err(unauth)
            }
        })
    };
}

pub fn basic_auth() -> Router<Arc<crate::AppState>> {
    Router::new()
        .route("/start", get(start::start))
        .route("/ip", get(ip::ip))
        .layer(require_auth!(
            &env::var("BASIC_TOKEN").expect("no `BASIC_TOKEN` env var.")
        ))
}

pub fn stop_auth() -> Router<Arc<crate::AppState>> {
    Router::new()
        .route("/stop", get(stop::stop))
        .route("/wake", get(wake::wake))
        .layer(require_auth!(
            &env::var("STOP_TOKEN").expect("no `STOP_TOKEN` env var.")
        ))
}
