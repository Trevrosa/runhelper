use rocket::{State, futures::SinkExt, get, tokio::sync::broadcast};
use rocket_ws::{Channel, WebSocket};

/// forward the websocket from the local runner.
#[get("/console")]
pub async fn console(
    ws: WebSocket,
    stats_channel: &State<broadcast::Sender<String>>,
) -> Channel<'static> {
    let mut stats_channel = stats_channel.subscribe();

    ws.channel(move |mut stream| {
        Box::pin(async move {
            while let Ok(message) = stats_channel.recv().await {
                if let Err(err) = stream.send(rocket_ws::Message::text(message)).await {
                    tracing::warn!("{err}, closing socket");
                    break;
                }
            }
            tracing::warn!("channel closed");
            Ok(())
        })
    })
}
