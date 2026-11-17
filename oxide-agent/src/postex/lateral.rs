use anyhow::Result;

pub async fn psexec(args: &[String]) -> Result<String> {
    // args: [target_host, command]
    if args.len() < 2 {
        return Err(anyhow::anyhow!("usage: psexec <host> <command>"));
    }
    let host    = &args[0];
    let command = &args[1..].join(" ");

    #[cfg(target_os = "windows")]
    {
        // Connect to ADMIN$ share, copy binary, create service, start, cleanup
        return Ok(format!("psexec stub: {host} → {command}"));
    }
    #[allow(unreachable_code)]
    Ok(format!("psexec stub: {host} → {command}"))
}

pub async fn wmi_exec(args: &[String]) -> Result<String> {
    // args: [target_host, command]
    if args.len() < 2 {
        return Err(anyhow::anyhow!("usage: wmi_exec <host> <command>"));
    }
    let host    = &args[0];
    let command = &args[1..].join(" ");

    #[cfg(target_os = "windows")]
    {
        // WMI Win32_Process.Create via COM — no network auth needed with current token
        let script = format!(
            "wmic /node:\"{host}\" process call create \"{command}\""
        );
        let out = tokio::process::Command::new("cmd")
            .args(["/c", &script])
            .output().await?;
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    #[allow(unreachable_code)]
    Ok(format!("wmi_exec stub: {host} → {command}"))
}
