use std::fs::OpenOptions;
use std::io::Write;

fn log_path() -> std::path::PathBuf {
    let tmp = std::env::var("TEMP")
        .or_else(|_| std::env::var("TMP"))
        .unwrap_or_else(|_| "C:\\Temp".into());
    std::path::PathBuf::from(tmp).join("oxide_debug.txt")
}

pub fn log(msg: &str) {
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{ts}] {msg}\n");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path()) {
        let _ = f.write_all(line.as_bytes());
    }
    // Also print to stderr in case a console is attached
    eprint!("{line}");
}

#[macro_export]
macro_rules! dbg_log {
    ($($arg:tt)*) => {
        $crate::debug::log(&format!($($arg)*))
    };
}
