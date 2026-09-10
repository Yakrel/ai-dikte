//! Native Windows message loop, tray and Win+Z interception.
//! Hook callbacks only enqueue events; audio/network work runs on another thread.
use crate::controller::{self, Command};
use anyhow::{Context, Result, bail};
use std::{
    cell::{Cell, RefCell},
    ptr::{null, null_mut},
    sync::{Mutex, OnceLock},
};
use tokio::sync::mpsc;
use windows_sys::Win32::{
    Foundation::*,
    System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
    UI::{Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
};
const TRAY_MESSAGE: u32 = WM_APP + 1;
const STATUS_MESSAGE: u32 = WM_APP + 2;
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
static COMMANDS: OnceLock<mpsc::Sender<Command>> = OnceLock::new();
static STATUS: Mutex<String> = Mutex::new(String::new());
thread_local! {
    static OSD: Cell<HWND> = const { Cell::new(null_mut()) };
    static CHORD: Cell<bool> = const { Cell::new(false) };
    static TRAY: RefCell<Option<NOTIFYICONDATAW>> = const { RefCell::new(None) };
}
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn copy_wide<const N: usize>(target: &mut [u16; N], text: &str) {
    target.fill(0);
    for (dest, unit) in target.iter_mut().take(N - 1).zip(text.encode_utf16()) {
        *dest = unit;
    }
}
fn enqueue(command: Command) {
    if let Some(tx) = COMMANDS.get() {
        let _ = tx.try_send(command);
    }
}
#[derive(Debug, PartialEq, Eq)]
enum ChordAction {
    Pass,
    Consume,
    Toggle,
}
fn chord_event(active: &mut bool, down: bool, up: bool, win: bool) -> ChordAction {
    if down && *active {
        // Keep swallowing repeats even when Win was released before Z.
        ChordAction::Consume
    } else if down && win {
        *active = true;
        ChordAction::Toggle
    } else if up && *active {
        *active = false;
        ChordAction::Consume
    } else {
        ChordAction::Pass
    }
}

unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 {
            let key = &*(lp as *const KBDLLHOOKSTRUCT);
            if key.flags & LLKHF_INJECTED == 0 && key.vkCode == VK_Z as u32 {
                let down = wp == WM_KEYDOWN as usize || wp == WM_SYSKEYDOWN as usize;
                let up = wp == WM_KEYUP as usize || wp == WM_SYSKEYUP as usize;
                let win =
                    GetAsyncKeyState(VK_LWIN as i32) < 0 || GetAsyncKeyState(VK_RWIN as i32) < 0;
                let action = CHORD.with(|chord| {
                    let mut active = chord.get();
                    let action = chord_event(&mut active, down, up, win);
                    chord.set(active);
                    action
                });
                if action == ChordAction::Toggle {
                    // Match the existing app's Start-menu suppression.
                    let mut events: [INPUT; 2] = std::mem::zeroed();
                    for (index, event) in events.iter_mut().enumerate() {
                        event.r#type = INPUT_KEYBOARD;
                        event.Anonymous.ki = KEYBDINPUT {
                            wVk: 0xFC,
                            wScan: 0,
                            dwFlags: if index == 0 { 0 } else { KEYEVENTF_KEYUP },
                            time: 0,
                            dwExtraInfo: 0,
                        };
                    }
                    SendInput(2, events.as_ptr(), std::mem::size_of::<INPUT>() as i32);
                    enqueue(Command::Toggle);
                }
                if action != ChordAction::Pass {
                    return 1;
                }
            }
        }
        CallNextHookEx(null_mut(), code, wp, lp)
    }
}
fn companion(name: &str) -> Result<std::path::PathBuf> {
    let path = std::env::current_exe()?.with_file_name(name);
    anyhow::ensure!(
        path.is_file(),
        "Missing {name}; reinstall the complete AI Dikte package"
    );
    Ok(path)
}
fn launch(command: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new(companion("ai-dikte.exe")?)
        .arg(command)
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_CONSOLE)
        .spawn()
        .context("Cannot open terminal menu")?;
    Ok(())
}
unsafe extern "system" fn window(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if TASKBAR_CREATED.get() == Some(&message) {
            TRAY.with_borrow_mut(|tray| {
                if let Some(tray) = tray {
                    tray.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
                    if Shell_NotifyIconW(NIM_ADD, tray) == 0 {
                        MessageBoxW(
                            hwnd,
                            wide("Cannot restore AI Dikte tray icon after Explorer restart")
                                .as_ptr(),
                            wide("AI Dikte").as_ptr(),
                            MB_ICONERROR,
                        );
                    }
                }
            });
            return 0;
        }
        match message {
            WM_TIMER => {
                OSD.with(|osd| {
                    ShowWindow(osd.get(), SW_HIDE);
                });
                KillTimer(hwnd, 1);
                0
            }
            TRAY_MESSAGE if lp as u32 == WM_LBUTTONDBLCLK => {
                enqueue(Command::Toggle);
                0
            }
            TRAY_MESSAGE if lp as u32 == WM_RBUTTONUP => {
                let menu = CreatePopupMenu();
                if !menu.is_null() {
                    AppendMenuW(menu, MF_STRING, 1, wide("Start / stop (Win+Z)").as_ptr());
                    AppendMenuW(menu, MF_STRING, 2, wide("Settings").as_ptr());
                    AppendMenuW(menu, MF_STRING, 4, wide("Diagnostics").as_ptr());
                    AppendMenuW(menu, MF_STRING, 5, wide("Session log").as_ptr());
                    let startup = startup_enabled().unwrap_or(false);
                    AppendMenuW(
                        menu,
                        MF_STRING | if startup { MF_CHECKED } else { 0 },
                        6,
                        wide("Start with Windows").as_ptr(),
                    );
                    AppendMenuW(menu, MF_STRING, 3, wide("Exit").as_ptr());
                    let mut point: POINT = std::mem::zeroed();
                    GetCursorPos(&mut point);
                    SetForegroundWindow(hwnd);
                    let selected = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_NONOTIFY,
                        point.x,
                        point.y,
                        0,
                        hwnd,
                        null(),
                    );
                    DestroyMenu(menu);
                    match selected {
                        1 => enqueue(Command::Toggle),
                        2 => {
                            if let Err(e) = launch("setup") {
                                MessageBoxW(
                                    hwnd,
                                    wide(&e.to_string()).as_ptr(),
                                    wide("AI Dikte").as_ptr(),
                                    MB_ICONERROR,
                                );
                            }
                        }
                        4 | 5 => {
                            if let Err(error) = launch(if selected == 4 {
                                "diagnostics"
                            } else {
                                "log-view"
                            }) {
                                MessageBoxW(
                                    hwnd,
                                    wide(&error.to_string()).as_ptr(),
                                    wide("AI Dikte").as_ptr(),
                                    MB_ICONERROR,
                                );
                            }
                        }
                        6 => {
                            if let Err(error) = set_startup(!startup) {
                                MessageBoxW(
                                    hwnd,
                                    wide(&error.to_string()).as_ptr(),
                                    wide("AI Dikte").as_ptr(),
                                    MB_ICONERROR,
                                );
                            }
                        }
                        3 => {
                            enqueue(Command::Quit);
                            PostQuitMessage(0);
                        }
                        _ => (),
                    }
                }
                0
            }
            STATUS_MESSAGE => {
                if let Ok(status) = STATUS.lock() {
                    TRAY.with_borrow_mut(|tray| {
                        if let Some(tray) = tray {
                            copy_wide(&mut tray.szTip, &format!("AI Dikte — {status}"));
                            copy_wide(&mut tray.szInfo, &status);
                            copy_wide(&mut tray.szInfoTitle, "AI Dikte");
                            let enabled = crate::config::path()
                                .and_then(|p| crate::config::Config::load(&p))
                                .map(|c| c.notify_mode == crate::config::Notifications::All)
                                .unwrap_or(true);
                            tray.uFlags = NIF_TIP
                                | if enabled || status.starts_with("Error:") {
                                    NIF_INFO
                                } else {
                                    0
                                };
                            tray.dwInfoFlags = if status.starts_with("Error:") {
                                NIIF_ERROR
                            } else {
                                NIIF_INFO
                            };
                            Shell_NotifyIconW(NIM_MODIFY, tray);
                            if enabled || status.starts_with("Error:") {
                                OSD.with(|osd| {
                                    SetWindowTextW(osd.get(), wide(&status).as_ptr());
                                    ShowWindow(osd.get(), SW_SHOWNOACTIVATE);
                                });
                                SetTimer(
                                    hwnd,
                                    1,
                                    if status.starts_with("Error:") {
                                        8000
                                    } else {
                                        2500
                                    },
                                    None,
                                );
                            }
                        }
                    });
                }
                0
            }
            WM_CLOSE => {
                enqueue(Command::Quit);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, wp, lp),
        }
    }
}
struct Resources {
    mutex: HANDLE,
    hook: HHOOK,
    window: HWND,
}
impl Drop for Resources {
    fn drop(&mut self) {
        unsafe {
            if !self.hook.is_null() {
                UnhookWindowsHookEx(self.hook);
            }
            TRAY.with_borrow_mut(|tray| {
                if let Some(data) = tray.take() {
                    Shell_NotifyIconW(NIM_DELETE, &data);
                }
            });
            OSD.with(|osd| {
                if !osd.get().is_null() {
                    DestroyWindow(osd.replace(null_mut()));
                }
            });
            if !self.window.is_null() {
                DestroyWindow(self.window);
            }
            if !self.mutex.is_null() {
                CloseHandle(self.mutex);
            }
        }
    }
}
pub fn daemon() -> Result<()> {
    unsafe {
        let mutex = CreateMutexW(null(), 0, wide("Local\\AI-Dikte-Daemon").as_ptr());
        if mutex.is_null() {
            bail!("Cannot create daemon mutex");
        }
        let mut resources = Resources {
            mutex,
            hook: null_mut(),
            window: null_mut(),
        };
        if GetLastError() == ERROR_ALREADY_EXISTS {
            bail!("AI Dikte is already running; exit the other daemon first");
        }
        let instance = GetModuleHandleW(null());
        let class_name = wide("AI-Dikte-Rust-Tray");
        let class = WNDCLASSW {
            lpfnWndProc: Some(window),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassW(&class) == 0 {
            bail!("Cannot register tray window");
        }
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide("AI Dikte").as_ptr(),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            bail!("Cannot create tray window");
        }
        resources.window = hwnd;
        let taskbar_message = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
        if taskbar_message == 0 {
            bail!("Cannot register Explorer restart notification");
        }
        let _ = TASKBAR_CREATED.set(taskbar_message);
        let mut area: RECT = std::mem::zeroed();
        if SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut area as *mut RECT).cast(), 0) == 0 {
            bail!("Cannot locate OSD work area");
        }
        let osd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            wide("STATIC").as_ptr(),
            wide("AI Dikte").as_ptr(),
            WS_POPUP | WS_BORDER | windows_sys::Win32::System::SystemServices::SS_CENTER,
            area.left + (area.right - area.left - 460) / 2,
            area.bottom - 100,
            460,
            50,
            hwnd,
            null_mut(),
            instance,
            null(),
        );
        if osd.is_null() {
            bail!("Cannot create recording status display");
        }
        OSD.with(|value| value.set(osd));

        let mut tray: NOTIFYICONDATAW = std::mem::zeroed();
        tray.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        tray.hWnd = hwnd;
        tray.uID = 1;
        tray.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        tray.uCallbackMessage = TRAY_MESSAGE;
        tray.hIcon = LoadIconW(instance, std::ptr::without_provenance::<u16>(1));
        if tray.hIcon.is_null() {
            bail!("Cannot load application icon");
        }
        copy_wide(&mut tray.szTip, "AI Dikte — Ready (Win+Z)");
        if Shell_NotifyIconW(NIM_ADD, &tray) == 0 {
            bail!("Cannot create system tray icon");
        }
        TRAY.with_borrow_mut(|value| *value = Some(tray));
        let (tx, rx) = mpsc::channel(16);
        COMMANDS
            .set(tx)
            .map_err(|_| anyhow::anyhow!("Daemon already initialized"))?;
        resources.hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), instance, 0);
        if resources.hook.is_null() {
            bail!("Cannot install Win+Z keyboard hook");
        }
        let window_id = hwnd as usize;
        let runtime = tokio::runtime::Runtime::new()?;
        let worker = std::thread::spawn(move || {
            runtime.block_on(controller::run(rx, |message| {
                if let Ok(mut status) = STATUS.lock() {
                    *status = message.to_owned();
                }
                PostMessageW(window_id as HWND, STATUS_MESSAGE, 0, 0);
            }))
        });
        let mut message: MSG = std::mem::zeroed();
        loop {
            let result = GetMessageW(&mut message, null_mut(), 0, 0);
            if result <= 0 {
                break;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        // The bounded hotkey queue may be full. Shutdown must not be lost,
        // otherwise joining the worker can wait forever.
        if let Some(tx) = COMMANDS.get() {
            let _ = tx.blocking_send(Command::Quit);
        }
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("Session worker panicked"))?;
        Ok(())
    }
}

pub fn set_startup(enabled: bool) -> Result<()> {
    use windows_sys::Win32::System::Registry::*;
    let startup_value = if enabled {
        Some(wide(&format!(
            "\"{}\"",
            companion("ai-dikte-background.exe")?.display()
        )))
    } else {
        None
    };
    unsafe {
        let mut key = null_mut();
        let result = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run").as_ptr(),
            0,
            null(),
            0,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        );
        if result != ERROR_SUCCESS {
            bail!("Cannot open startup registry key ({result})");
        }
        let result = if enabled {
            let value = startup_value.as_ref().unwrap();
            RegSetValueExW(
                key,
                wide("AI-Dikte").as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        } else {
            RegDeleteValueW(key, wide("AI-Dikte").as_ptr())
        };
        RegCloseKey(key);
        if result != ERROR_SUCCESS && !(result == ERROR_FILE_NOT_FOUND && !enabled) {
            bail!("Cannot update startup setting ({result})");
        }
        Ok(())
    }
}
pub fn startup_enabled() -> Result<bool> {
    use windows_sys::Win32::System::Registry::*;
    let mut bytes = 0;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run").as_ptr(),
            wide("AI-Dikte").as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut bytes,
        )
    };
    match result {
        ERROR_SUCCESS => Ok(true),
        ERROR_FILE_NOT_FOUND => Ok(false),
        _ => bail!("Cannot read startup setting ({result})"),
    }
}

pub fn running() -> Result<bool> {
    let handle = unsafe {
        windows_sys::Win32::System::Threading::OpenMutexW(
            0x00100000, // SYNCHRONIZE: only inspect whether the daemon mutex exists.
            0,
            wide("Local\\AI-Dikte-Daemon").as_ptr(),
        )
    };
    if !handle.is_null() {
        unsafe {
            CloseHandle(handle);
        }
        return Ok(true);
    }
    let error = unsafe { GetLastError() };
    if error == ERROR_FILE_NOT_FOUND {
        Ok(false)
    } else {
        bail!("Cannot inspect background app ({error})")
    }
}

pub fn ensure_background() -> Result<()> {
    use std::os::windows::process::CommandExt;
    if running()? {
        return Ok(());
    }
    std::process::Command::new(companion("ai-dikte-background.exe")?)
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
        .spawn()
        .context("Cannot start background app")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_released_before_z_does_not_leak_repeat_or_toggle_twice() {
        let mut active = false;
        assert_eq!(
            chord_event(&mut active, true, false, true),
            ChordAction::Toggle
        );
        for win in [true, false, false] {
            assert_eq!(
                chord_event(&mut active, true, false, win),
                ChordAction::Consume
            );
        }
        assert_eq!(
            chord_event(&mut active, false, true, false),
            ChordAction::Consume
        );
        assert_eq!(
            chord_event(&mut active, true, false, false),
            ChordAction::Pass
        );
        assert_eq!(
            chord_event(&mut active, false, true, false),
            ChordAction::Pass
        );
        assert_eq!(
            chord_event(&mut active, true, false, true),
            ChordAction::Toggle
        );
    }
}
