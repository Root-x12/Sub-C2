mod build;
mod config;
mod db;
mod listeners;
mod operators;
mod state;

use anyhow::Result;
use clap::Parser;
use tracing::info;

#[derive(Parser)]
#[command(name = "oxide-server", version, about = "OXIDE C2 Team Server")]
struct Args {
    /// Config file path
    #[arg(short, long, default_value = "oxide-server.toml")]
    config: String,
    /// Generate a new config and exit
    #[arg(long)]
    init: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oxide_server=info".into()),
        )
        .init();

    let args = Args::parse();
    let cfg = config::load_or_init(&args.config, args.init)?;

    if args.init {
        info!("Config written to {}", args.config);
        return Ok(());
    }

    info!("OXIDE C2 Server v{}", env!("CARGO_PKG_VERSION"));
    info!("Operator bind: {}:{}", cfg.operator_host, cfg.operator_port);

    let db = db::init(&cfg.db_path).await?;
    let shared = state::AppState::new(cfg.clone(), db);

    // Start listener manager
    listeners::Manager::new(shared.clone()).start().await;

    // Start operator WebSocket server
    operators::Server::new(shared.clone())
        .run(&cfg.operator_host, cfg.operator_port)
        .await?;

    Ok(())
}
