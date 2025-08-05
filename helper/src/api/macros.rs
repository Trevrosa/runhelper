// $route is required because the #[get] macro expects a literal, meaning i cant use stringify!($route) or any macro in its input.
macro_rules! make_forward {
    ($name:ident, $route:expr, $auth:ty) => {
        pub mod $name {
            use reqwest::Client;
            use rocket::{State, get, http::Status};

            use crate::{RUNNER_ADDR, UrlExt};

            #[get($route)]
            pub async fn $name(
                _auth: $auth,
                client: &State<Client>,
            ) -> Result<(Status, String), Status> {
                let $name = client
                    .get(RUNNER_ADDR.join_unchecked(stringify!($name)))
                    .send()
                    .await
                    .map_err(|_| Status::ServiceUnavailable)?;
                let status = $name.status();
                let $name = $name
                    .text()
                    .await
                    .map_err(|_| Status::InternalServerError)?;

                Ok((Status::new(status.as_u16()), $name))
            }
        }
    };
    ($name:ident, $route:expr) => {
        pub mod $name {
            use reqwest::Client;
            use rocket::{State, get, http::Status};

            use crate::{RUNNER_ADDR, UrlExt};

            #[get($route)]
            pub async fn $name(client: &State<Client>) -> Result<(Status, String), Status> {
                let $name = client
                    .get(RUNNER_ADDR.join_unchecked(stringify!($name)))
                    .send()
                    .await
                    .map_err(|_| Status::ServiceUnavailable)?;
                let status = $name.status();
                let $name = $name
                    .text()
                    .await
                    .map_err(|_| Status::InternalServerError)?;

                Ok((Status::new(status.as_u16()), $name))
            }
        }
    };
}
