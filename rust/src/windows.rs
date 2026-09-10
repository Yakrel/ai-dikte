//! Native Windows message loop, OSD capsule and Win+Z interception.
//! Headless daemon without system tray; managed via CLI or Start Menu.
use crate::controller::{self, Command};
use anyhow::{Context, Result, bail};
use std::{
    cell::Cell,
    ptr::{null, null_mut},
    sync::{Mutex, OnceLock},
};
use tokio::sync::mpsc;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{
        Console::{ATTACH_PARENT_PROCESS, AllocConsole, AttachConsole},
        LibraryLoader::GetModuleHandleW,
        Registry::*,
        Threading::{
            CreateEventW, CreateMutexW, INFINITE, OpenEventW, OpenMutexW, SetEvent,
            WaitForSingleObject,
        },
    },
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

const STATUS_MESSAGE: u32 = WM_APP + 2;
static COMMANDS: OnceLock<mpsc::Sender<Command>> = OnceLock::new();
static OSD_STATE: Mutex<(u32, String)> = Mutex::new((0x00A1E3A6, String::new()));

thread_local! {
    static OSD_HWND: Cell<HWND> = const { Cell::new(null_mut()) };
    static CHORD: Cell<bool> = const { Cell::new(false) };
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

pub fn ensure_console() {
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            AllocConsole();
        }
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
                    // Suppress Windows 11 Snap Layouts popup.
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

unsafe extern "system" fn osd_proc(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_TIMER => {
                ShowWindow(hwnd, SW_HIDE);
                KillTimer(hwnd, 1);
                0
            }
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);

                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);

                // Background: #181825 (BGR: 0x00251818)
                let bg_brush = CreateSolidBrush(0x00251818);
                // Border: #313244 (BGR: 0x00443231)
                let border_pen = CreatePen(PS_SOLID as i32, 1, 0x00443231);

                let old_brush = SelectObject(hdc, bg_brush);
                let old_pen = SelectObject(hdc, border_pen);

                RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, 12, 12);

                SelectObject(hdc, old_brush);
                SelectObject(hdc, old_pen);
                DeleteObject(bg_brush);
                DeleteObject(border_pen);

                SetBkMode(hdc, TRANSPARENT as i32);

                let font_name = wide("Segoe UI");
                let font = CreateFontW(
                    -14,
                    0,
                    0,
                    0,
                    FW_BOLD as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY,
                    DEFAULT_PITCH | FF_DONTCARE,
                    font_name.as_ptr(),
                );
                let old_font = SelectObject(hdc, font);

                let (dot_color, text) = OSD_STATE.lock().unwrap().clone();

                // Colored status indicator dot: ●
                SetTextColor(hdc, dot_color);
                let dot = wide("●");
                TextOutW(hdc, 16, 11, dot.as_ptr(), 1);

                // Text: #cdd6f4 (BGR: 0x00F4D6CD)
                SetTextColor(hdc, 0x00F4D6CD);
                let wtext = wide(&text);
                TextOutW(hdc, 34, 11, wtext.as_ptr(), (wtext.len() - 1) as i32);

                SelectObject(hdc, old_font);
                DeleteObject(font);

                EndPaint(hwnd, &ps);
                0
            }
            _ => DefWindowProcW(hwnd, message, wp, lp),
        }
    }
}

unsafe extern "system" fn window(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match message {
            STATUS_MESSAGE => {
                let osd = OSD_HWND.with(|h| h.get());
                if !osd.is_null() {
                    InvalidateRect(osd, null(), 1);
                    ShowWindow(osd, SW_SHOWNOACTIVATE);
                    if wp > 0 {
                        SetTimer(osd, 1, wp as u32, None);
                    } else {
                        KillTimer(osd, 1);
                    }
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
    stop_event: HANDLE,
    hook: HHOOK,
    window: HWND,
    osd: HWND,
}

impl Drop for Resources {
    fn drop(&mut self) {
        unsafe {
            if !self.hook.is_null() {
                UnhookWindowsHookEx(self.hook);
            }
            if !self.osd.is_null() {
                DestroyWindow(self.osd);
            }
            if !self.window.is_null() {
                DestroyWindow(self.window);
            }
            if !self.stop_event.is_null() {
                CloseHandle(self.stop_event);
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
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(mutex);
            bail!("AI Dikte background service is already running.");
        }
        if stop_event.is_null() {
            CloseHandle(mutex);
            bail!("Cannot create daemon stop event");
        }

        let mut resources = Resources {
            mutex,
            stop_event,
            hook: null_mut(),
            window: null_mut(),
            osd: null_mut(),
        };

        let instance = GetModuleHandleW(null());
        let class_name = wide("AI-Dikte-Rust-Daemon");
        let class = WNDCLASSW {
            lpfnWndProc: Some(window),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&class);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide("AI Dikte Daemon").as_ptr(),
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
            bail!("Cannot create daemon message window");
        }
        resources.window = hwnd;

        // Register OSD floating capsule class and window
        let osd_class_name = wide("AI-Dikte-OSD");
        let osd_class = WNDCLASSW {
            lpfnWndProc: Some(osd_proc),
            hInstance: instance,
            lpszClassName: osd_class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&osd_class);

        let mut area: RECT = std::mem::zeroed();
        if SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut area as *mut RECT).cast(), 0) == 0 {
            area.right = 1920;
            area.bottom = 1080;
        }

        let osd_width = 240;
        let osd_height = 40;
        let osd_x = area.right - osd_width - 24;
        let osd_y = area.bottom - osd_height - 24;

        let osd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            osd_class_name.as_ptr(),
            wide("AI Dikte OSD").as_ptr(),
            WS_POPUP,
            osd_x,
            osd_y,
            osd_width,
            osd_height,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if osd.is_null() {
            bail!("Cannot create OSD capsule window");
        }
        resources.osd = osd;
        OSD_HWND.with(|h| h.set(osd));

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
            runtime.block_on(controller::run(rx, move |message| {
                let (dot_color, display_text, auto_hide_ms) = if message.starts_with("Recording") {
                    (0x00A88BF3, "Listening...".to_string(), None)
                } else if message.starts_with("Finishing") {
                    (0x00AFE2F9, "Transcribing...".to_string(), None)
                } else if message == "Ready" {
                    (0x00A1E3A6, "Text inserted".to_string(), Some(1200))
                } else if message == "No speech detected" {
                    (0x00AFE2F9, "No speech detected".to_string(), Some(1500))
                } else if message.starts_with("Error:") {
                    let msg = &message[6..message.len().min(40)];
                    (0x00A88BF3, format!("Error: {msg}"), Some(3500))
                } else {
                    (0x00A1E3A6, message.to_string(), Some(1500))
                };
                if let Ok(mut state) = OSD_STATE.lock() {
                    *state = (dot_color, display_text);
                }
                let hide_ms = auto_hide_ms.unwrap_or(0) as usize;
                unsafe {
                    PostMessageW(window_id as HWND, STATUS_MESSAGE, hide_ms, 0);
                }
            }))
        });

        // Dedicated thread listening for stop event
        let stop_event_raw = stop_event as usize;
        std::thread::spawn(move || {
            WaitForSingleObject(stop_event_raw as HANDLE, INFINITE);
            PostMessageW(window_id as HWND, WM_CLOSE, 0, 0);
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

        if let Some(tx) = COMMANDS.get() {
            let _ = tx.blocking_send(Command::Quit);
        }
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("Session worker panicked"))?;

        Ok(())
    }
}

pub fn stop_daemon() -> Result<()> {
    unsafe {
        let event = OpenEventW(
            windows_sys::Win32::System::Threading::EVENT_MODIFY_STATE,
            0,
            wide("Local\\AI-Dikte-Daemon-Stop").as_ptr(),
        );
        if event.is_null() {
            bail!("AI Dikte background service is not running.");
        }
        SetEvent(event);
        CloseHandle(event);

        // Wait up to 3 seconds for the daemon to exit
        for _ in 0..30 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if !running()? {
                return Ok(());
            }
        }
        bail!("Background service did not respond to stop signal.");
    }
}

pub fn set_startup(enabled: bool) -> Result<()> {
    let startup_value = if enabled {
        Some(wide(&format!(
            "\"{}\" daemon",
            std::env::current_exe()?.display()
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
        OpenMutexW(
            0x00100000, // SYNCHRONIZE
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
    std::process::Command::new(std::env::current_exe()?)
        .arg("daemon")
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
