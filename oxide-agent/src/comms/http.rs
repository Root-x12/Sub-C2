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
        self.client
            .post(format!("{}/register", self.base_url))
            .json(reg)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn beacon(&self, b: &AgentBeacon) -> Result<BeaconResponse> {
        let resp = self.client
            .post(format!("{}/beacon/{}", self.base_url, self.agent_id))
            .json(b)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }
}
