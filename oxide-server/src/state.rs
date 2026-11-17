use crate::config::ServerConfig;
use oxide_common::types::*;
use oxide_common::protocol::SrvMessage;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Per-agent channel: server pushes task results to subscribed operators.
pub type AgentTx = broadcast::Sender<SrvMessage>;

pub struct Inner {
    pub config:    ServerConfig,
    pub db:        SqlitePool,
    pub agents:    HashMap<Uuid, AgentInfo>,
    pub listeners: HashMap<Uuid, ListenerInfo>,
    pub loot:      Vec<LootEntry>,
    pub tasks:     HashMap<Uuid, Vec<crate::state::PendingTask>>,
    /// Broadcast channel — all operator connections subscribe.
    pub broadcast: broadcast::Sender<SrvMessage>,
}

#[derive(Debug, Clone)]
pub struct PendingTask {
    pub task: oxide_common::types::AgentTask,
}

#[derive(Clone)]
pub struct AppState(pub Arc<RwLock<Inner>>);

impl AppState {
    pub fn new(config: ServerConfig, db: SqlitePool) -> Self {
        let (tx, _) = broadcast::channel(512);
        let inner = Inner {
            config,
            db,
            agents:    HashMap::new(),
            listeners: HashMap::new(),
            loot:      Vec::new(),
            tasks:     HashMap::new(),
            broadcast: tx,
        };
        AppState(Arc::new(RwLock::new(inner)))
    }

    pub async fn broadcast(&self, msg: SrvMessage) {
        let inner = self.0.read().await;
        let _ = inner.broadcast.send(msg);
    }

    pub async fn upsert_agent(&self, info: AgentInfo) {
        let mut inner = self.0.write().await;
        let is_new = !inner.agents.contains_key(&info.id);
        let msg = if is_new {
            SrvMessage::AgentNew { agent: info.clone() }
        } else {
            SrvMessage::AgentUpdate { agent: info.clone() }
        };
        inner.agents.insert(info.id, info);
        let _ = inner.broadcast.send(msg);
    }

    pub async fn queue_task(&self, agent_id: Uuid, task: oxide_common::types::AgentTask) {
        let mut inner = self.0.write().await;
        inner.tasks.entry(agent_id).or_default().push(PendingTask { task });
    }

    pub async fn drain_tasks(&self, agent_id: &Uuid) -> Vec<oxide_common::types::AgentTask> {
        let mut inner = self.0.write().await;
        inner.tasks
            .remove(agent_id)
            .unwrap_or_default()
            .into_iter()
            .map(|pt| pt.task)
            .collect()
    }

    pub async fn add_loot(&self, entry: LootEntry) {
        let mut inner = self.0.write().await;
        inner.loot.push(entry);
    }

    pub async fn listener_online(&self, info: ListenerInfo) {
        let mut inner = self.0.write().await;
        inner.listeners.insert(info.config.id, info);
    }

    pub async fn listener_offline(&self, id: Uuid) {
        let mut inner = self.0.write().await;
        if let Some(l) = inner.listeners.get_mut(&id) {
            l.status = ListenerStatus::Stopped;
        }
    }
}
