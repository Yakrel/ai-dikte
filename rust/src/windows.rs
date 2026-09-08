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
static COMMANDS: OnceLock<mpsc::Sender<Command>> = OnceLock::new();
static STATUS: Mutex<String> = Mutex::new(String::new());
thread_local! {
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
unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 {
            let key = &*(lp as *const KBDLLHOOKSTRUCT);
            if key.flags & LLKHF_INJECTED == 0 && key.vkCode == VK_Z as u32 {
                let down = wp == WM_KEYDOWN as usize || wp == WM_SYSKEYDOWN as usize;
                let up = wp == WM_KEYUP as usize || wp == WM_SYSKEYUP as usize;
                let win =
                    GetAsyncKeyState(VK_LWIN as i32) < 0 || GetAsyncKeyState(VK_RWIN as i32) < 0;
                if down && win {
                    if !CHORD.replace(true) {
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
                    return 1;
                }
                // Consume Z release even if Win was released first.
                if up && CHORD.replace(false) {
                    return 1;
                }
            }
        }
        CallNextHookEx(null_mut(), code, wp, lp)
    }
}
fn launch_setup() -> Result<()> {
    std::process::Command::new(std::env::current_exe()?)
        .arg("setup")
        .spawn()
        .context("Cannot open settings")?;
    Ok(())
}
unsafe extern "system" fn window(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match message {
            TRAY_MESSAGE if lp as u32 == WM_LBUTTONDBLCLK => {
                enqueue(Command::Toggle);
                0
            }
            TRAY_MESSAGE if lp as u32 == WM_RBUTTONUP => {
                let menu = CreatePopupMenu();
                if !menu.is_null() {
                    AppendMenuW(menu, MF_STRING, 1, wide("Start / stop (Win+Z)").as_ptr());
                    AppendMenuW(menu, MF_STRING, 2, wide("Settings").as_ptr());
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
                            if let Err(e) = launch_setup() {
                                MessageBoxW(
                                    hwnd,
                                    wide(&e.to_string()).as_ptr(),
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
        let mut tray: NOTIFYICONDATAW = std::mem::zeroed();
        tray.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        tray.hWnd = hwnd;
        tray.uID = 1;
        tray.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        tray.uCallbackMessage = TRAY_MESSAGE;
        tray.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);
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
        enqueue(Command::Quit);
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("Session worker panicked"))?;
        Ok(())
    }
}
