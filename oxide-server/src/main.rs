mod build;
mod config;
mod db;
mod listeners;
mod operators;
mod state;

use anyhow::Result;
use clap::Parser;
use oxide_common::{protocol::SrvMessage, types::AgentStatus};
use tracing::info;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "oxide-server", version, about = "OXIDE C2 Team Server")]
struct Args {
    /// Config file path
    #[arg(short, long, default_value = "oxide-server.toml")]
    config: String,
    /// Generate a new config and exit
    #[arg(long)]
    init: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oxide_server=info".into()),
        )
        .init();

    let args = Args::parse();
    let cfg = config::load_or_init(&args.config, args.init)?;

    if args.init {
        info!("Config written to {}", args.config);
        return Ok(());
    }

    info!("OXIDE C2 Server v{}", env!("CARGO_PKG_VERSION"));
    info!("Operator bind: {}:{}", cfg.operator_host, cfg.operator_port);

    let db = db::init(&cfg.db_path).await?;
    let shared = state::AppState::new(cfg.clone(), db);

    // Start heartbeat watchdog — marks stale/dead agents and evicts confirmed dead ones
    let watchdog_state = shared.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
            heartbeat_check(&watchdog_state).await;
        }
    });

    // Start listener manager
    listeners::Manager::new(shared.clone()).start().await;

    // Start operator WebSocket server
    operators::Server::new(shared.clone())
        .run(&cfg.operator_host, cfg.operator_port)
        .await?;

    Ok(())
}

/// Every 15 seconds: advance missed_beats, mark Stale after 5 misses,
/// evict and broadcast AgentDead after 20 misses.
async fn heartbeat_check(state: &state::AppState) {
    let now = chrono::Utc::now();
    let mut to_dead: Vec<Uuid> = Vec::new();
    let mut to_broadcast: Vec<SrvMessage> = Vec::new();

    {
        let mut inner = state.0.write().await;
        for (id, agent) in inner.agents.iter_mut() {
            let elapsed_ms = (now - agent.last_seen).num_milliseconds().max(0) as u64;
            let beat_ms = agent.sleep_ms.max(1000); // floor at 1 s to avoid div-by-zero
            let missed = elapsed_ms / beat_ms;
            agent.missed_beats = missed as u32;

            match agent.status {
                AgentStatus::Active if missed >= 5 => {
                    agent.status = AgentStatus::Stale;
                    info!("agent {} ({}) went STALE ({}ms since last beacon)",
                        agent.hostname, id, elapsed_ms);
                    to_broadcast.push(SrvMessage::AgentUpdate { agent: agent.clone() });
                }
                AgentStatus::Stale if missed >= 20 => {
                    info!("agent {} ({}) declared DEAD ({}ms)", agent.hostname, id, elapsed_ms);
                    to_dead.push(*id);
                }
                _ => {}
            }
        }
        for id in &to_dead {
            inner.agents.remove(id);
        }
    }

    for msg in to_broadcast {
        let _ = state.0.read().await.broadcast.send(msg);
    }
    for id in to_dead {
        let _ = state.0.read().await.broadcast
            .send(SrvMessage::AgentDead { agent_id: id });
    }
}
