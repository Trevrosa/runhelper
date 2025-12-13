// use rocket::{State, futures::SinkExt, get, tokio::sync::broadcast};
// use rocket_ws::{Channel, WebSocket};

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use tokio::sync::broadcast::Receiver;

use super::AppState;

/// forward the websocket from the local runner.
pub async fn console(ws: WebSocketUpgrade, State(state): AppState) -> Response {
    let channel = state.console.subscribe();
    ws.on_upgrade(|socket| handle_socket(socket, channel))
}

async fn handle_socket(mut socket: WebSocket, mut channel: Receiver<String>) {
    loop {
        while let Ok(message) = channel.recv().await {
            if let Err(err) = socket.send(Message::Text(message.into())).await {
                tracing::warn!("{err}, closing socket");
                break;
            }
        }
        tracing::warn!("ws closed");
    }
}

// #[get("/console")]
// pub async fn console(
//     ws: WebSocket,
//     stats_channel: &State<broadcast::Sender<String>>,
// ) -> Channel<'static> {
//     let mut stats_channel = stats_channel.subscribe();

//     ws.channel(move |mut stream| {
//         Box::pin(async move {
//             while let Ok(message) = stats_channel.recv().await {
//                 if let Err(err) = stream.send(rocket_ws::Message::text(message)).await {
//                     tracing::warn!("{err}, closing socket");
//                     break;
//                 }
//             }
//             tracing::warn!("ws closed");
//             Ok(())
//         })
//     })
// }
