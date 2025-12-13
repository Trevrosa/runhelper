// $route is required because the #[get] macro expects a literal, meaning i cant use stringify!($route) or any macro in its input.
macro_rules! make_forward {
    ($name:ident, $route:expr) => {
        pub mod $name {
            use axum::http::StatusCode;
            use helper::UrlExt;

            use super::AppState;
            use crate::RUNNER_ADDR;

            pub async fn $name(
                axum::extract::State(state): AppState,
            ) -> Result<(StatusCode, String), StatusCode> {
                let $name = state
                    .client
                    .get(RUNNER_ADDR.join_unchecked(stringify!($name)))
                    .send()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let status = axum::http::StatusCode::from_u16($name.status().as_u16())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let $name = $name
                    .text()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                Ok((status, $name))
            }
        }
    };
}
