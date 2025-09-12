use std::sync::LazyLock;

use rocket::{
    Request,
    http::Status,
    request::{FromRequest, Outcome},
};

/// allows access to /start, /ip, /wake
pub static BASIC_TOKEN: LazyLock<String> =
    LazyLock::new(|| std::env::var("BASIC_TOKEN").expect("no `BASIC_TOKEN` env var."));
/// allows access to /stop
pub static STOP_TOKEN: LazyLock<String> =
    LazyLock::new(|| std::env::var("STOP_TOKEN").expect("no `STOP_TOKEN` env var."));

pub struct BasicAuth(());

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BasicAuth {
    type Error = &'static str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = req.headers().get_one("token");

        match token {
            Some(token) => {
                if token == *BASIC_TOKEN {
                    Outcome::Success(Self(()))
                } else {
                    Outcome::Error((Status::Unauthorized, "failed, not authorized"))
                }
            }
            None => Outcome::Error((Status::BadRequest, "token was not found")),
        }
    }
}

pub struct StopAuth(());

#[rocket::async_trait]
impl<'r> FromRequest<'r> for StopAuth {
    type Error = &'static str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = req.headers().get_one("token");

        match token {
            Some(token) => {
                if token == *STOP_TOKEN {
                    Outcome::Success(Self(()))
                } else {
                    Outcome::Error((Status::Unauthorized, "failed, not authorized"))
                }
            }
            None => Outcome::Error((Status::BadRequest, "token was not found")),
        }
    }
}
