use std::sync::Arc;

use reqwest::{Client, Url};
use rocket::{State, get, http::Status};

use crate::UrlExt;

#[get("/stop")]
pub async fn stop(client: &State<Client>, runner_addr: &State<Arc<Url>>) -> Result<String, Status> {
    let resp = client
        .get(runner_addr.join_unchecked("stop"))
        .send()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    resp.text().await.map_err(|_| Status::InternalServerError)
}
