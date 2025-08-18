use common::Stats;
use reqwest_websocket::Bytes;
use rocket::{State, futures::SinkExt, get, tokio::sync::broadcast};
use rocket_ws::{Channel, WebSocket};

/// forward the websocket from the local runner.
#[get("/stats")]
pub async fn stats(
    ws: WebSocket,
    stats_channel: &State<broadcast::Sender<Bytes>>,
) -> Channel<'static> {
    let mut stats_channel = stats_channel.subscribe();

    ws.channel(move |mut stream| {
        Box::pin(async move {
            while let Ok(message) = stats_channel.recv().await {
                let Ok(stats) = bitcode::deserialize::<Stats>(&message) else {
                    tracing::warn!("failed to deserialize bitcode");
                    continue;
                };
                let Ok(message) = serde_json::to_string(&stats) else {
                    tracing::warn!("failed to serialize to json");
                    continue;
                };

                if let Err(err) = stream.send(rocket_ws::Message::text(message)).await {
                    tracing::warn!("{err}, closing socket");
                    break;
                }
            }
            tracing::warn!("ws closed");
            Ok(())
        })
    })
}
