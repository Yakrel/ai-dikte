#![cfg_attr(windows, windows_subsystem = "windows")]
// Console-free Windows host. The TUI and daemon share the same library.
#[cfg(windows)]
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--self-test") {
        std::process::exit(if ai_dikte::diagnostics::self_test().is_ok() {
            0
        } else {
            1
        });
    }
    let result =
        ai_dikte::diagnostics::check_configuration().and_then(|()| ai_dikte::windows::daemon());
    if let Err(error) = result {
        let text: Vec<u16> = format!("AI Dikte: {error:#}\0").encode_utf16().collect();
        let title: Vec<u16> = "AI Dikte\0".encode_utf16().collect();
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
        std::process::exit(1);
    }
}
#[cfg(not(windows))]
fn main() {
    eprintln!("This host is only used on Windows; use ai-dikte daemon on Linux.");
    std::process::exit(1);
}
