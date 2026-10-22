//! Native Windows message loop, OSD capsule and Win+Z interception.
//! Headless daemon without system tray; managed via CLI.
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
            CreateEventW, CreateMutexW, INFINITE, OpenEventW, OpenMutexW, ResetEvent, SetEvent,
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

fn chord_event(
    active: &mut bool,
    down: bool,
    up: bool,
    win: bool,
    extra_modifiers: bool,
) -> ChordAction {
    if down && *active {
        ChordAction::Consume
    } else if down && win && !extra_modifiers {
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
                let extra_modifiers = GetAsyncKeyState(VK_CONTROL as i32) < 0
                    || GetAsyncKeyState(VK_MENU as i32) < 0
                    || GetAsyncKeyState(VK_SHIFT as i32) < 0;
                let action = CHORD.with(|chord| {
                    let mut active = chord.get();
                    let action = chord_event(&mut active, down, up, win, extra_modifiers);
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
                FillRect(hdc, &rect, bg_brush);
                // Border: #313244 (BGR: 0x00443231)
                let border_pen = CreatePen(PS_SOLID, 1, 0x00443231);

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
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    CLEARTYPE_QUALITY as u32,
                    (DEFAULT_PITCH | FF_DONTCARE) as u32,
                    font_name.as_ptr(),
                );
                let old_font = SelectObject(hdc, font);

                let (dot_color, text) = OSD_STATE.lock().unwrap().clone();

                let wtext = wide(&text);
                let text_len = (wtext.len() - 1) as i32;
                let mut extent: SIZE = std::mem::zeroed();
                GetTextExtentPoint32W(hdc, wtext.as_ptr(), text_len, &mut extent);
                // Center the indicator + text as one group; long errors are
                // ellipsized inside the same padded area instead of overflowing.
                let dot_size = 8;
                let gap = 10;
                let padding = 16;
                let text_width = extent
                    .cx
                    .min((rect.right - rect.left - 2 * padding - dot_size - gap).max(0));
                let left = rect.left + (rect.right - rect.left - dot_size - gap - text_width) / 2;
                let dot_top = rect.top + (rect.bottom - rect.top - dot_size) / 2;
                let dot_brush = CreateSolidBrush(dot_color);
                let old_brush = SelectObject(hdc, dot_brush);
                let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN));
                Ellipse(hdc, left, dot_top, left + dot_size, dot_top + dot_size);
                SelectObject(hdc, old_brush);
                SelectObject(hdc, old_pen);
                DeleteObject(dot_brush);

                SetTextColor(hdc, 0x00F4D6CD);
                let mut text_rect = RECT {
                    left: left + dot_size + gap,
                    top: rect.top,
                    right: left + dot_size + gap + text_width,
                    bottom: rect.bottom,
                };
                DrawTextW(
                    hdc,
                    wtext.as_ptr(),
                    text_len,
                    &mut text_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
                );

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
    ready_event: HANDLE,
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
            if !self.ready_event.is_null() {
                CloseHandle(self.ready_event);
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
        let stop_event = CreateEventW(null(), 0, 0, wide("Local\\AI-Dikte-Daemon-Stop").as_ptr());
        if stop_event.is_null() {
            CloseHandle(mutex);
            bail!("Cannot create daemon stop event");
        }

        let mut resources = Resources {
            mutex,
            stop_event,
            ready_event: null_mut(),
            hook: null_mut(),
            window: null_mut(),
            osd: null_mut(),
        };
        resources.ready_event =
            CreateEventW(null(), 1, 0, wide("Local\\AI-Dikte-Daemon-Ready").as_ptr());
        if resources.ready_event.is_null() || ResetEvent(resources.ready_event) == 0 {
            bail!("Cannot create daemon readiness event");
        }

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
        // RoundRect only rounds the paint; the native window must also be clipped
        // so its otherwise unpainted rectangular corners never cover the desktop.
        let region = CreateRoundRectRgn(0, 0, osd_width, osd_height, 12, 12);
        if region.is_null() {
            bail!("Cannot create rounded OSD region");
        }
        if SetWindowRgn(osd, region, 1) == 0 {
            DeleteObject(region);
            bail!("Cannot apply rounded OSD region");
        }
        // The window owns the region after a successful SetWindowRgn.
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
                    (0x00A88BF3, error_display_text(message), Some(3500))
                } else {
                    (0x00A1E3A6, message.to_string(), Some(1500))
                };
                if let Ok(mut state) = OSD_STATE.lock() {
                    *state = (dot_color, display_text);
                }
                let hide_ms = auto_hide_ms.unwrap_or(0) as usize;
                PostMessageW(window_id as HWND, STATUS_MESSAGE, hide_ms, 0);
            }))
        });

        // Dedicated thread listening for stop event
        let stop_event_raw = stop_event as usize;
        let stop_waiter = std::thread::spawn(move || {
            if WaitForSingleObject(stop_event_raw as HANDLE, INFINITE) == WAIT_OBJECT_0 {
                PostMessageW(window_id as HWND, WM_CLOSE, 0, 0);
            }
        });

        let mut message: MSG = std::mem::zeroed();
        let loop_result = if SetEvent(resources.ready_event) == 0 {
            Err(anyhow::anyhow!("Cannot signal daemon readiness"))
        } else {
            loop {
                let result = GetMessageW(&mut message, null_mut(), 0, 0);
                if result == -1 {
                    break Err(anyhow::anyhow!(
                        "Daemon message loop failed: {}",
                        std::io::Error::last_os_error()
                    ));
                }
                if result == 0 {
                    break Ok(());
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        };
        ResetEvent(resources.ready_event);
        // Wake and join the waiter before closing the handle it is waiting on.
        SetEvent(stop_event);
        let waiter_result = stop_waiter.join();

        if let Some(tx) = COMMANDS.get() {
            let _ = tx.blocking_send(Command::Quit);
        }
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("Session worker panicked"))?;

        waiter_result.map_err(|_| anyhow::anyhow!("Stop-event worker panicked"))?;
        loop_result
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
        let result = SetEvent(event);
        CloseHandle(event);
        if result == 0 {
            bail!("Cannot signal daemon stop event");
        }

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

fn installed_dir() -> Result<std::path::PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .context("LOCALAPPDATA is missing")?;
    if !root.is_absolute() {
        bail!("LOCALAPPDATA must be an absolute path");
    }
    Ok(root.join("Programs").join("AI-Dikte"))
}

pub fn installed_exe_path() -> Result<std::path::PathBuf> {
    Ok(installed_dir()?.join("ai-dikte.exe"))
}

fn same_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a
            .to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(b.to_string_lossy().trim_end_matches(['\\', '/'])),
    }
}

pub fn is_running_from_installed_path() -> Result<bool> {
    let current = std::env::current_exe()?;
    let installed = installed_exe_path()?;
    Ok(installed.exists() && same_path(&current, &installed))
}

/// Prefer the canonical installed binary whenever it exists on disk so running a
/// downloaded copy never repoints sign-in startup or spawns the daemon from Downloads.
fn preferred_daemon_exe() -> Result<std::path::PathBuf> {
    let installed = installed_exe_path()?;
    if installed.is_file() {
        Ok(installed)
    } else {
        Ok(std::env::current_exe()?)
    }
}

pub fn set_startup(enabled: bool) -> Result<()> {
    let startup_value = if enabled {
        Some(wide(&format!(
            "\"{}\" daemon",
            preferred_daemon_exe()?.display()
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
    use std::{os::windows::process::CommandExt, process::Stdio, time::Duration};
    let mut child = if running()? {
        None
    } else {
        Some(
            std::process::Command::new(preferred_daemon_exe()?)
                .arg("daemon")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
                .spawn()
                .context("Cannot start background app")?,
        )
    };
    for _ in 0..50 {
        if daemon_ready()? {
            return Ok(());
        }
        if let Some(process) = &mut child
            && let Some(status) = process
                .try_wait()
                .context("Cannot inspect background app")?
        {
            if !running()? {
                bail!("Background app exited before becoming ready ({status})");
            }
            // Another concurrent starter may have won the singleton race.
            child = None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if let Some(mut child) = child {
        child
            .kill()
            .context("Cannot stop unresponsive background app")?;
        child
            .wait()
            .context("Cannot reap unresponsive background app")?;
    }
    bail!("Background app did not become ready within 5 seconds")
}

fn daemon_ready() -> Result<bool> {
    unsafe {
        let event = OpenEventW(
            0x00100000, // SYNCHRONIZE
            0,
            wide("Local\\AI-Dikte-Daemon-Ready").as_ptr(),
        );
        if event.is_null() {
            let error = GetLastError();
            if error == ERROR_FILE_NOT_FOUND {
                return Ok(false);
            }
            bail!("Cannot inspect background app readiness ({error})");
        }
        let result = WaitForSingleObject(event, 0);
        CloseHandle(event);
        match result {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => bail!("Cannot wait for background app readiness"),
        }
    }
}

fn error_display_text(message: &str) -> String {
    let detail = message.strip_prefix("Error:").unwrap_or(message);
    let end = detail
        .char_indices()
        .nth(34)
        .map_or(detail.len(), |(index, _)| index);
    format!("Error: {}", &detail[..end])
}

fn merge_user_path(current: &str, preferred: &str, remove: &[&str]) -> String {
    let mut parts = vec![preferred.to_string()];
    for raw in current.split(';') {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let norm = trimmed.trim_end_matches(['\\', '/']);
        let skip = std::iter::once(preferred)
            .chain(remove.iter().copied())
            .any(|candidate| norm.eq_ignore_ascii_case(candidate.trim_end_matches(['\\', '/'])));
        if !skip {
            parts.push(trimmed.to_string());
        }
    }
    parts.join(";")
}

fn ensure_user_path(preferred: &std::path::Path, remove: &[&std::path::Path]) -> Result<()> {
    let preferred_str = preferred
        .to_str()
        .context("Installation path is not valid UTF-8")?;
    let remove_strs: Vec<&str> = remove.iter().filter_map(|p| p.to_str()).collect();
    unsafe {
        let mut key = null_mut();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide("Environment").as_ptr(),
            0,
            null(),
            0,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        );
        if status != ERROR_SUCCESS {
            bail!("Cannot open user Environment registry key ({status})");
        }

        let value_name = wide("Path");
        let mut value_type: u32 = REG_EXPAND_SZ;
        let mut bytes: u32 = 0;
        let query = RegQueryValueExW(
            key,
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            null_mut(),
            &mut bytes,
        );
        let current = if query == ERROR_FILE_NOT_FOUND {
            value_type = REG_EXPAND_SZ;
            String::new()
        } else if query != ERROR_SUCCESS {
            RegCloseKey(key);
            bail!("Cannot read user PATH from registry ({query})");
        } else {
            let mut buf = vec![0u16; (bytes as usize / 2) + 1];
            let mut read_bytes = (buf.len() * 2) as u32;
            let query = RegQueryValueExW(
                key,
                value_name.as_ptr(),
                null_mut(),
                &mut value_type,
                buf.as_mut_ptr().cast(),
                &mut read_bytes,
            );
            if query != ERROR_SUCCESS {
                RegCloseKey(key);
                bail!("Cannot read user PATH buffer ({query})");
            }
            let len = (read_bytes as usize / 2).min(buf.len());
            let slice = &buf[..len];
            let end = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
            String::from_utf16_lossy(&slice[..end])
        };

        let updated = merge_user_path(&current, preferred_str, &remove_strs);
        let encoded = wide(&updated);
        let reg_type = if value_type == REG_SZ {
            REG_SZ
        } else {
            REG_EXPAND_SZ
        };
        let set = RegSetValueExW(
            key,
            value_name.as_ptr(),
            0,
            reg_type,
            encoded.as_ptr().cast(),
            (encoded.len() * 2) as u32,
        );
        RegCloseKey(key);
        if set != ERROR_SUCCESS {
            bail!("Cannot update user PATH in registry ({set})");
        }

        let env_wide = wide("Environment");
        let mut _result: usize = 0;
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            env_wide.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            1000,
            &mut _result,
        );
    }
    Ok(())
}

fn write_command_launcher(install_dir: &std::path::Path) -> Result<std::path::PathBuf> {
    let bin = install_dir.join("bin");
    std::fs::create_dir_all(&bin).context("Cannot create bin launcher directory")?;
    let launcher_path = bin.join("ai-dikte.cmd");
    let contents = "@echo off\r\n\"%~dp0..\\ai-dikte.exe\" %*\r\nexit /b %errorlevel%\r\n";
    std::fs::write(&launcher_path, contents).context("Cannot write ai-dikte.cmd launcher")?;
    Ok(bin)
}

/// Installs the currently running executable into `%LOCALAPPDATA%\Programs\AI-Dikte`
/// (if running from elsewhere) and repairs the CLI launcher, user PATH, sign-in
/// startup entry, and background service.
pub fn install_or_repair() -> Result<std::path::PathBuf> {
    let install_dir = installed_dir()?;
    let target_exe = install_dir.join("ai-dikte.exe");
    let current_exe = std::env::current_exe().context("Cannot resolve current executable path")?;

    if running()? {
        let _ = stop_daemon();
    }

    std::fs::create_dir_all(&install_dir).context("Cannot create installation directory")?;

    if !(target_exe.exists() && same_path(&current_exe, &target_exe)) {
        let staged = install_dir.join("ai-dikte.exe.new");
        std::fs::copy(&current_exe, &staged).with_context(|| {
            format!(
                "Cannot copy {} to {}",
                current_exe.display(),
                staged.display()
            )
        })?;
        let mut last_err = None;
        for _ in 0..20 {
            match std::fs::rename(&staged, &target_exe) {
                Ok(()) => {
                    last_err = None;
                    break;
                }
                Err(err) => {
                    last_err = Some(err);
                    std::thread::sleep(std::time::Duration::from_millis(150));
                }
            }
        }
        if let Some(err) = last_err {
            let _ = std::fs::remove_file(&staged);
            return Err(err).with_context(|| {
                format!(
                    "Cannot replace {}; close any other running AI Dikte windows and retry",
                    target_exe.display()
                )
            });
        }
    }

    let bin_dir = write_command_launcher(&install_dir)?;
    ensure_user_path(&bin_dir, &[install_dir.as_path()])?;
    set_startup(true)?;

    // Start the daemon only when configuration + API key are already present;
    // otherwise the user is still pre-onboarding and `setup` will start it.
    if crate::diagnostics::check_daemon().is_ok() {
        ensure_background()?;
    }

    Ok(target_exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_error_messages_are_truncated_without_panicking() {
        let detail = "ş".repeat(40);
        let displayed = error_display_text(&format!("Error:{detail}"));
        assert_eq!(displayed, format!("Error: {}", "ş".repeat(34)));
        assert_eq!(error_display_text("Error:短い"), "Error: 短い");
    }

    #[test]
    fn win_released_before_z_does_not_leak_repeat_or_toggle_twice() {
        let mut active = false;
        assert_eq!(
            chord_event(&mut active, true, false, true, false),
            ChordAction::Toggle
        );
        for win in [true, false, false] {
            assert_eq!(
                chord_event(&mut active, true, false, win, false),
                ChordAction::Consume
            );
        }
        assert_eq!(
            chord_event(&mut active, false, true, false, false),
            ChordAction::Consume
        );
        assert_eq!(
            chord_event(&mut active, true, false, false, false),
            ChordAction::Pass
        );
        assert_eq!(
            chord_event(&mut active, false, true, false, false),
            ChordAction::Pass
        );
        assert_eq!(
            chord_event(&mut active, true, false, true, false),
            ChordAction::Toggle
        );
    }

    #[test]
    fn modified_win_z_passes_through_without_activating() {
        let mut active = false;
        assert_eq!(
            chord_event(&mut active, true, false, true, true),
            ChordAction::Pass
        );
        assert!(!active);
        assert_eq!(
            chord_event(&mut active, false, true, true, true),
            ChordAction::Pass
        );
        assert_eq!(
            chord_event(&mut active, true, false, true, false),
            ChordAction::Toggle
        );
    }

    #[test]
    fn modifiers_pressed_during_active_chord_do_not_leak_repeat_or_release() {
        let mut active = false;
        assert_eq!(
            chord_event(&mut active, true, false, true, false),
            ChordAction::Toggle
        );
        assert_eq!(
            chord_event(&mut active, true, false, true, true),
            ChordAction::Consume
        );
        assert_eq!(
            chord_event(&mut active, false, true, false, true),
            ChordAction::Consume
        );
        assert_eq!(
            chord_event(&mut active, true, false, true, true),
            ChordAction::Pass
        );
    }

    #[test]
    fn user_path_merge_prepends_bin_and_removes_duplicates_case_insensitively() {
        let merged = merge_user_path(
            r"C:\Windows\System32;c:\users\u\appdata\local\programs\ai-dikte\bin\;C:\Users\u\AppData\Local\Programs\AI-Dikte;C:\Tools",
            r"C:\Users\u\AppData\Local\Programs\AI-Dikte\bin",
            &[r"C:\Users\u\AppData\Local\Programs\AI-Dikte"],
        );
        assert_eq!(
            merged,
            r"C:\Users\u\AppData\Local\Programs\AI-Dikte\bin;C:\Windows\System32;C:\Tools"
        );
    }
}
