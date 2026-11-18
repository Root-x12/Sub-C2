//! HTTP/HTTPS listener — receives agent beacons and returns task queues.

use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use chrono::Utc;
use oxide_common::{
    protocol::{AgentBeacon, AgentRegister, BeaconResponse, SrvMessage},
    types::{AgentInfo, AgentStatus, ListenerConfig},
};
use tracing::info;
use uuid::Uuid;

pub async fn run(cfg: ListenerConfig, state: AppState) -> anyhow::Result<()> {
    let router = Router::new()
        .route("/register", post(handle_register))
        .route("/beacon/:agent_id", post(handle_beacon))
        .with_state(state);

    // Always bind on all interfaces; cfg.host is the advertised address for agents
    let addr: std::net::SocketAddr =
        format!("0.0.0.0:{}", cfg.port).parse()?;

    info!("HTTP listener on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn handle_register(
    State(state): State<AppState>,
    Json(reg): Json<AgentRegister>,
) -> impl IntoResponse {
    info!("New agent: {} @ {} ({})", reg.hostname, reg.ip, reg.privilege);

    let now = Utc::now();
    let info = AgentInfo {
        id:           reg.agent_id,
        hostname:     reg.hostname,
        username:     reg.username,
        domain:       reg.domain,
        ip:           reg.ip,
        os:           reg.os,
        arch:         reg.arch,
        pid:          reg.pid,
        privilege:    reg.privilege,
        listener_id:  reg.listener_id,
        build_id:     reg.build_id,
        sleep_ms:     reg.sleep_ms,
        jitter_pct:   reg.jitter_pct,
        status:       AgentStatus::Active,
        first_seen:   now,
        last_seen:    now,
        missed_beats: 0,
    };

    state.upsert_agent(info).await;
    StatusCode::OK
}

async fn handle_beacon(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    body: Bytes,
) -> impl IntoResponse {
    // Decrypt if needed — for now accept plaintext JSON
    let beacon: AgentBeacon = match serde_json::from_slice(&body) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("bad beacon from {agent_id}: {e}");
            return (StatusCode::BAD_REQUEST, Bytes::new());
        }
    };

    // Update last_seen and broadcast AgentUpdate so the TUI sparkline/graph reflects live beacons
    let updated_agent = {
        let mut inner = state.0.write().await;
        if let Some(a) = inner.agents.get_mut(&agent_id) {
            a.last_seen = Utc::now();
            a.status = AgentStatus::Active;
            a.missed_beats = 0;
            Some(a.clone())
        } else {
            None
        }
    };
    if let Some(agent) = updated_agent {
        let _ = state.0.read().await.broadcast
            .send(oxide_common::protocol::SrvMessage::AgentUpdate { agent });
    }

    // Push results to operators (fill in agent_id from URL path)
    for mut result in beacon.results {
        result.agent_id = agent_id;
        info!("task result: task={} agent={} output_bytes={} err={:?}",
            result.task_id, agent_id, result.output.len(), result.error);
        let _ = state.0.read().await.broadcast
            .send(SrvMessage::TaskResult { result });
    }

    // Return pending tasks
    let tasks = state.drain_tasks(&agent_id).await;
    if !tasks.is_empty() {
        info!("dispatching {} tasks to agent {}", tasks.len(), agent_id);
    }
    let resp = BeaconResponse {
        tasks,
        kill: false,
        new_sleep: None,
    };

    match serde_json::to_vec(&resp) {
        Ok(body) => (StatusCode::OK, Bytes::from(body)),
        Err(_)   => (StatusCode::INTERNAL_SERVER_ERROR, Bytes::new()),
    }
}
