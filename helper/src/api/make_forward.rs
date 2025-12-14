use axum::{http::StatusCode, response::IntoResponse};

pub struct Error;

const ERROR: (StatusCode, &str) = (
    StatusCode::INTERNAL_SERVER_ERROR,
    "couldn't forward the request",
);

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        ERROR.into_response()
    }
}

// $route is required because the #[get] macro expects a literal, meaning i cant use stringify!($route) or any macro in its input.
macro_rules! make_forward {
    ($name:ident, $route:expr) => {
        pub mod $name {
            use axum::http::StatusCode;
            use helper::UrlExt;

            use super::{AppState, make_forward::Error};
            use crate::RUNNER_ADDR;

            pub async fn $name(
                axum::extract::State(state): AppState,
            ) -> Result<(StatusCode, String), Error> {
                let $name = state
                    .client
                    .get(RUNNER_ADDR.join_unchecked(stringify!($name)))
                    .send()
                    .await
                    .map_err(|_| Error)?;
                let status = StatusCode::from_u16($name.status().as_u16()).map_err(|_| Error)?;
                let $name = $name.text().await.map_err(|_| Error)?;

                Ok((status, $name))
            }
        }
    };
}
