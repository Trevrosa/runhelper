use std::sync::Arc;

use reqwest::{Client, Url};
use rocket::{State, get, http::Status};

use crate::UrlExt;

#[get("/run")]
pub async fn run(client: &State<Client>, runner_addr: &State<Arc<Url>>) -> Result<String, Status> {
    let resp = client
        .get(runner_addr.join_unchecked("run"))
        .send()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    resp.text().await.map_err(|_| Status::InternalServerError)
}
