use std::sync::Arc;

use reqwest::{Client, Url};
use rocket::{State, get, http::Status};

use crate::UrlExt;

#[get("/ip")]
pub async fn ip(client: &State<Client>, runner_addr: &State<Arc<Url>>) -> Result<String, Status> {
    let ip = client
        .get(runner_addr.join_unchecked("ip"))
        .send()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    ip.text().await.map_err(|_| Status::InternalServerError)
}
