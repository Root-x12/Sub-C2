//! WebSocket client — connects to oxide-server, handles auth.

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use oxide_common::protocol::{OpMessage, SrvMessage};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
#[allow(unused_imports)]
use url::Url;

pub async fn connect(
    addr:  &str,
    token: &str,
) -> Result<(mpsc::Sender<OpMessage>, mpsc::Receiver<SrvMessage>)> {
    let (ws, _) = connect_async(addr).await?;
    let (mut sink, mut stream) = ws.split();

    // Channels
    let (op_tx,  mut op_rx)  = mpsc::channel::<OpMessage>(64);
    let (srv_tx, srv_rx)     = mpsc::channel::<SrvMessage>(256);

    // Send auth
    let auth = serde_json::to_string(&OpMessage::Auth { token: token.to_string() })?;
    sink.send(Message::Text(auth.into())).await?;

    // Spawn send task
    tokio::spawn(async move {
        while let Some(op) = op_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&op) {
                if sink.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Spawn receive task
    let token_clone = token.to_string();
    tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if let Message::Text(text) = msg {
                if let Ok(srv) = serde_json::from_str::<SrvMessage>(&text) {
                    if srv_tx.send(srv).await.is_err() {
                        break;
                    }
                }
            }
        }
        drop(token_clone);
    });

    Ok((op_tx, srv_rx))
}
