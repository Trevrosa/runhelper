use rocket::{
    Request,
    http::Status,
    request::{FromRequest, Outcome},
};

/// allows access to /start, /ip, /wake
const BASIC_TOKEN: &str = include_str!("../basic_token");

/// allows access to /stop
const STOP_TOKEN: &str = include_str!("../stop_token");

pub struct BasicAuth(());

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BasicAuth {
    type Error = &'static str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = req.headers().get_one("Authorization");

        match token {
            Some(token) => {
                if token.contains(BASIC_TOKEN) {
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
        let token = req.headers().get_one("Authorization");

        match token {
            Some(token) => {
                if token.contains(STOP_TOKEN) {
                    Outcome::Success(Self(()))
                } else {
                    Outcome::Error((Status::Unauthorized, "failed, not authorized"))
                }
            }
            None => Outcome::Error((Status::BadRequest, "token was not found")),
        }
    }
}
