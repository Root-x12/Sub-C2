use anyhow::Result;
use oxide_common::protocol::{AgentBeacon, AgentRegister, BeaconResponse, TCP_MAX_MSG};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use uuid::Uuid;

pub struct TcpTransport {
    host:     String,
    port:     u16,
    agent_id: Uuid,
}

impl TcpTransport {
    pub fn new(host: &str, port: u16, agent_id: Uuid) -> Self {
        TcpTransport {
            host: host.to_string(),
            port,
            agent_id,
        }
    }

    pub async fn register(&self, reg: &AgentRegister) -> Result<()> {
        let body = serde_json::to_vec(reg)?;
        let _resp = self.send_recv(&body).await?;
        Ok(())
    }

    pub async fn beacon(&self, b: &AgentBeacon) -> Result<BeaconResponse> {
        let body = serde_json::to_vec(b)?;
        let resp = self.send_recv(&body).await?;
        Ok(serde_json::from_slice(&resp)?)
    }

    async fn send_recv(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect(&addr).await?;

        // Write length-prefixed message
        let len = (payload.len() as u32).to_be_bytes();
        stream.write_all(&len).await?;
        stream.write_all(payload).await?;

        // Read response
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let resp_len = u32::from_be_bytes(len_buf) as usize;

        if resp_len > TCP_MAX_MSG {
            anyhow::bail!("response too large: {resp_len}");
        }

        let mut resp = vec![0u8; resp_len];
        stream.read_exact(&mut resp).await?;
        Ok(resp)
    }
}
