pub mod http;
pub mod tcp;

use anyhow::Result;
use oxide_common::protocol::{AgentBeacon, AgentRegister, BeaconResponse};

pub enum Transport {
    Http(http::HttpTransport),
    Tcp(tcp::TcpTransport),
}

impl Transport {
    pub async fn register(&mut self, reg: &AgentRegister) -> Result<()> {
        match self {
            Transport::Http(t) => t.register(reg).await,
            Transport::Tcp(t)  => t.register(reg).await,
        }
    }

    pub async fn beacon(&mut self, b: &AgentBeacon) -> Result<BeaconResponse> {
        match self {
            Transport::Http(t) => t.beacon(b).await,
            Transport::Tcp(t)  => t.beacon(b).await,
        }
    }
}
