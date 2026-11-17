use anyhow::{Context, Result};
use oxide_common::crypto::{generate_x25519_keypair, pubkey_to_hex};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub operator_host:   String,
    pub operator_port:   u16,
    pub operator_token:  String,
    pub db_path:         String,
    pub server_privkey:  String,
    pub server_pubkey:   String,
    pub tls_cert:        Option<String>,
    pub tls_key:         Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let (sk, pk) = generate_x25519_keypair();
        let privkey = hex::encode(sk.to_bytes());
        let pubkey = pubkey_to_hex(&pk);

        let token: String = {
            use rand::RngCore;
            let mut b = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut b);
            hex::encode(b)
        };

        ServerConfig {
            operator_host:  "127.0.0.1".into(),
            operator_port:  7878,
            operator_token: token,
            db_path:        "oxide-server.db".into(),
            server_privkey: privkey,
            server_pubkey:  pubkey,
            tls_cert:       None,
            tls_key:        None,
        }
    }
}

pub fn load_or_init(path: &str, init: bool) -> Result<ServerConfig> {
    if init || !Path::new(path).exists() {
        let cfg = ServerConfig::default();
        let content = toml::to_string_pretty(&cfg)
            .context("serialize config")?;
        std::fs::write(path, content).context("write config")?;
        println!("[+] Config generated at {path}");
        println!("[+] Server pubkey:  {}", cfg.server_pubkey);
        println!("[+] Operator token: {}", cfg.operator_token);
        return Ok(cfg);
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("read {path}"))?;
    toml::from_str(&content).context("parse config")
}
