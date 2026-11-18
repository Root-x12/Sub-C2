pub mod http;
pub mod tcp;

use crate::state::AppState;
use oxide_common::types::{ListenerProtocol, ListenerStatus};
use tracing::{error, info};

pub struct Manager {
    state: AppState,
}

impl Manager {
    pub fn new(state: AppState) -> Self {
        Manager { state }
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            loop {
                let configs = {
                    let inner = self.state.0.read().await;
                    inner.listeners
                        .values()
                        .filter(|l| l.status == ListenerStatus::Stopped) // Disabled is not restarted
                        .map(|l| l.config.clone())
                        .collect::<Vec<_>>()
                };

                for cfg in configs {
                    let state_clone = self.state.clone();
                    let cfg_clone = cfg.clone();
                    info!("Starting listener {} ({}) on {}:{}", cfg.name, cfg.protocol, cfg.host, cfg.port);

                    match cfg.protocol {
                        ListenerProtocol::Http | ListenerProtocol::Https => {
                            self.state.0.write().await
                                .listeners.get_mut(&cfg.id)
                                .map(|l| l.status = ListenerStatus::Running);
                            tokio::spawn(async move {
                                if let Err(e) = http::run(cfg_clone, state_clone).await {
                                    error!("HTTP listener error: {e}");
                                }
                            });
                        }
                        ListenerProtocol::Tcp => {
                            self.state.0.write().await
                                .listeners.get_mut(&cfg.id)
                                .map(|l| l.status = ListenerStatus::Running);
                            tokio::spawn(async move {
                                if let Err(e) = tcp::run(cfg_clone, state_clone).await {
                                    error!("TCP listener error: {e}");
                                }
                            });
                        }
                        _ => {
                            // DNS, GitHub, Discord — Phase 8
                            info!("Listener type {} not yet started (Phase 8)", cfg.protocol);
                        }
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
    }
}
