//! Crash log + panic hook.
//!
//! On a Rust panic, Tauri normally lets the process abort silently. For a
//! tray-bar app the user just sees the icon vanish, with no idea why. We
//! install a custom `panic::set_hook` that:
//!
//! 1. Writes a `panic-<unix-timestamp>.log` file under
//!    `%TEMP%\codex-runway-windows\` with the backtrace and message.
//! 2. Pops a Windows `MessageBox` so the user sees the failure even if
//!    they launched the portable .exe by double-clicking.
//! 3. Falls back to the default hook on non-Windows targets (we still
//!    log, just no popup).

use std::io::Write;
use std::path::PathBuf;

#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

/// Directory for crash logs. Created on first use.
pub fn crash_dir() -> PathBuf {
    let base = std::env::var_os("TEMP")
        .or_else(|| std::env::var_os("TMP"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("codex-runway-windows");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Install the global panic hook. Call exactly once at the very top of
/// `main()` / `run()`.
pub fn install() {
    std::panic::set_hook(Box::new(|info| {
        let payload = info.payload();
        let msg = if let Some(s) = payload.downcast_ref::<&'static str>() {
            (*s).to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown location>".to_string());
        let backtrace = std::backtrace::Backtrace::force_capture();
        let body = format!(
            "Codex Runway crashed.\n\n\
             Panic message: {msg}\n\
             At: {location}\n\
             Time (UTC unix): {}\n\n\
             Backtrace:\n{backtrace}\n",
            chrono::Utc::now().timestamp()
        );

        // Always try to write the log file first. Even on a popup failure
        // the user can find the log under %TEMP%\codex-runway-windows\.
        let path = crash_dir().join(format!("panic-{}.log", chrono::Utc::now().timestamp()));
        if let Ok(mut f) = std::fs::File::create(&path) {
            let _ = f.write_all(body.as_bytes());
        }

        // Forward to the default hook so any attached debugger still
        // sees the panic, and so the log channel keeps working.
        let _ = std::panic::catch_unwind(|| {
            eprintln!("{body}");
        });

        show_popup(&body);
    }));
}

#[cfg(windows)]
fn show_popup(body: &str) {
    // Truncate the message — MessageBoxW caps the displayed text; the
    // full content lives in the log file we just wrote.
    let title: Vec<u16> = "Codex Runway crashed\0".encode_utf16().collect();
    let summary = format!(
        "Codex Runway hit an unexpected error and is about to close.\n\n\
         A crash log was written to:\n{}\n\n\
         First line of the message:\n{}\n",
        crash_dir().display(),
        body.lines().next().unwrap_or(""),
    );
    let text: Vec<u16> = summary.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

#[cfg(not(windows))]
fn show_popup(body: &str) {
    eprintln!("[crash] {body}");
}
