// $route is required because the #[get] macro expects a literal, meaning i cant use stringify!($route) or any macro in its input.
macro_rules! make_forward {
    ($name:ident, $route:expr) => {
        pub mod $name {
            use reqwest::Client;
            use rocket::{State, get, http::Status};

            use crate::{UrlExt, RUNNER_ADDR};

            #[get($route)]
            pub async fn $name(
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
}

make_forward!(start, "/start");

make_forward!(stop, "/stop");

make_forward!(ip, "/ip");

make_forward!(ping, "/ping");

make_forward!(list, "/list");

pub mod stats;

pub mod console;

pub mod wake;
