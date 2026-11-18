//! WebSocket client — connects to oxide-server, handles auth and reconnect.

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use oxide_common::protocol::{OpMessage, SrvMessage};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Connect (or reconnect) to the operator WebSocket.
/// Returns a sender for outbound operator messages and a receiver for server messages.
/// The receiver yields `None` when the connection drops — the caller should reconnect.
pub async fn connect(
    addr:  &str,
    token: &str,
) -> Result<(mpsc::Sender<OpMessage>, mpsc::Receiver<SrvMessage>)> {
    let (ws, _) = connect_async(addr).await?;
    let (mut sink, mut stream) = ws.split();

    let (op_tx,  mut op_rx)  = mpsc::channel::<OpMessage>(64);
    let (srv_tx, srv_rx)     = mpsc::channel::<SrvMessage>(512);

    // Authenticate
    let auth = serde_json::to_string(&OpMessage::Auth { token: token.to_string() })?;
    sink.send(Message::Text(auth.into())).await?;

    // Outbound pump: TUI → server
    let srv_tx_clone = srv_tx.clone();
    tokio::spawn(async move {
        while let Some(op) = op_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&op) {
                if sink.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
        // Signal the inbound side that the connection is gone
        drop(srv_tx_clone);
    });

    // Inbound pump: server → TUI
    tokio::spawn(async move {
        while let Some(res) = stream.next().await {
            match res {
                Ok(Message::Text(text)) => {
                    if let Ok(srv) = serde_json::from_str::<SrvMessage>(&text) {
                        if srv_tx.send(srv).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    // pong is handled automatically by tungstenite
                    let _ = data;
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        // Channel closes → main loop sees None on next recv
    });

    Ok((op_tx, srv_rx))
}
