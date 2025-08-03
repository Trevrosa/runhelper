use std::{sync::Arc, time::Duration};

use common::Stats;
use reqwest::{Client, Url};
use reqwest_websocket::RequestBuilderExt;
use rocket::{
    State,
    futures::{SinkExt, StreamExt},
    get,
    http::Status,
};
use rocket_ws::{Channel, WebSocket};

use crate::UrlExt;

/// basically forward the websocket from the local runner.
#[get("/stats")]
pub async fn stats(
    ws: WebSocket,
    client: &State<Client>,
    runner_addr: &State<Arc<Url>>,
) -> Result<Channel<'static>, Status> {
    let resp = client
        .get(runner_addr.join_unchecked("stats"))
        .timeout(Duration::from_secs(4))
        .upgrade()
        .send()
        .await;
    let Ok(resp) = resp else {
        tracing::warn!("failed to send request");
        return Err(Status::ServiceUnavailable);
    };

    let runner_ws = resp.into_websocket().await;
    let Ok(mut runner_ws) = runner_ws else {
        tracing::warn!("failed to upgrade to websocket");
        return Err(Status::InternalServerError);
    };

    Ok(ws.channel(move |mut stream| {
        Box::pin(async move {
            while let Some(message) = runner_ws.next().await {
                let Ok(message) = message else {
                    tracing::warn!("websocket closed by runner");
                    break;
                };

                if let reqwest_websocket::Message::Binary(stats) = message {
                    let Ok(stats) = bitcode::deserialize::<Stats>(&stats) else {
                        tracing::warn!("failed to deserialize bitcode");
                        continue;
                    };
                    let Ok(message) = serde_json::to_string(&stats) else {
                        tracing::warn!("failed to deserialize bitcode");
                        continue;
                    };

                    if let Err(err) = stream.send(rocket_ws::Message::text(message)).await {
                        tracing::warn!("{err}, closing socket");
                        break;
                    }
                } else {
                    tracing::warn!("expected binary message, got something else");
                }
            }
            Ok(())
        })
    }))
}
