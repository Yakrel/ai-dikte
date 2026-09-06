"""Windows integration: credentials, Unicode input, Win+Z and recording overlay."""
from __future__ import annotations
import ctypes
from ctypes import wintypes
import queue
import sys
import threading
import time

APP = "ai-dikte"
IS_WINDOWS = sys.platform == "win32"
CREDENTIAL_TARGET = "Yakrel/AI-Dikte/GoogleAI"
_GLOBAL_HOOK_CB = None
_GLOBAL_HOOK_HANDLE = None
_GLOBAL_HOOK_THREAD = None
_GLOBAL_TOGGLE_QUEUE = None
_GLOBAL_CONTROL_WORKER_THREAD = None

def _windows_credential_api():
    if not IS_WINDOWS:
        raise RuntimeError("Windows Credential Manager is only available on Windows.")

    class CREDENTIALW(ctypes.Structure):
        _fields_ = [
            ("Flags", wintypes.DWORD),
            ("Type", wintypes.DWORD),
            ("TargetName", wintypes.LPWSTR),
            ("Comment", wintypes.LPWSTR),
            ("LastWritten", wintypes.FILETIME),
            ("CredentialBlobSize", wintypes.DWORD),
            ("CredentialBlob", ctypes.POINTER(ctypes.c_ubyte)),
            ("Persist", wintypes.DWORD),
            ("AttributeCount", wintypes.DWORD),
            ("Attributes", ctypes.c_void_p),
            ("TargetAlias", wintypes.LPWSTR),
            ("UserName", wintypes.LPWSTR),
        ]

    advapi32 = ctypes.WinDLL("Advapi32.dll", use_last_error=True)
    credential_pointer = ctypes.POINTER(CREDENTIALW)
    advapi32.CredReadW.argtypes = [
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        ctypes.POINTER(credential_pointer),
    ]
    advapi32.CredReadW.restype = wintypes.BOOL
    advapi32.CredWriteW.argtypes = [ctypes.POINTER(CREDENTIALW), wintypes.DWORD]
    advapi32.CredWriteW.restype = wintypes.BOOL
    advapi32.CredFree.argtypes = [ctypes.c_void_p]
    advapi32.CredFree.restype = None
    return advapi32, CREDENTIALW


def read_windows_api_key() -> str:
    """Read the API key from the current user's Windows Credential Manager."""
    advapi32, credential_type = _windows_credential_api()
    credential_pointer = ctypes.POINTER(credential_type)()
    if not advapi32.CredReadW(
        CREDENTIAL_TARGET,
        1,  # CRED_TYPE_GENERIC
        0,
        ctypes.byref(credential_pointer),
    ):
        error = ctypes.get_last_error()
        if error == 1168:  # ERROR_NOT_FOUND
            return ""
        raise OSError(error, ctypes.FormatError(error))

    try:
        credential = credential_pointer.contents
        blob = ctypes.string_at(
            credential.CredentialBlob,
            credential.CredentialBlobSize,
        )
        return blob.decode("utf-8").strip()
    finally:
        advapi32.CredFree(credential_pointer)


def write_windows_api_key(api_key: str) -> None:
    """Persist the API key as a generic per-user Windows credential."""
    key_bytes = api_key.strip().encode("utf-8")
    if not key_bytes:
        raise RuntimeError("API key cannot be empty.")
    if len(key_bytes) > 2560:
        raise RuntimeError("API key exceeds the Windows credential size limit.")

    advapi32, credential_type = _windows_credential_api()
    blob = ctypes.create_string_buffer(key_bytes)
    credential = credential_type()
    credential.Type = 1  # CRED_TYPE_GENERIC
    credential.TargetName = CREDENTIAL_TARGET
    credential.CredentialBlobSize = len(key_bytes)
    credential.CredentialBlob = ctypes.cast(blob, ctypes.POINTER(ctypes.c_ubyte))
    credential.Persist = 2  # CRED_PERSIST_LOCAL_MACHINE
    credential.UserName = APP

    if not advapi32.CredWriteW(ctypes.byref(credential), 0):
        error = ctypes.get_last_error()
        raise OSError(error, ctypes.FormatError(error))


def output_text_windows(text: str) -> str:
    """Inject text directly into active window using Win32 SendInput KEYEVENTF_UNICODE."""
    time.sleep(0.08)

    KEYEVENTF_UNICODE = 0x0004
    KEYEVENTF_KEYUP = 0x0002
    INPUT_KEYBOARD = 1
    ULONG_PTR = ctypes.c_ulonglong if ctypes.sizeof(ctypes.c_void_p) == 8 else wintypes.DWORD

    class KEYBDINPUT(ctypes.Structure):
        _fields_ = [
            ("wVk", wintypes.WORD),
            ("wScan", wintypes.WORD),
            ("dwFlags", wintypes.DWORD),
            ("time", wintypes.DWORD),
            ("dwExtraInfo", ULONG_PTR),
        ]

    class HARDWAREINPUT(ctypes.Structure):
        _fields_ = [
            ("uMsg", wintypes.DWORD),
            ("wParamL", wintypes.WORD),
            ("wParamH", wintypes.WORD),
        ]

    class MOUSEINPUT(ctypes.Structure):
        _fields_ = [
            ("dx", wintypes.LONG),
            ("dy", wintypes.LONG),
            ("mouseData", wintypes.DWORD),
            ("dwFlags", wintypes.DWORD),
            ("time", wintypes.DWORD),
            ("dwExtraInfo", ULONG_PTR),
        ]

    class INPUT_UNION(ctypes.Union):
        _fields_ = [
            ("mi", MOUSEINPUT),
            ("ki", KEYBDINPUT),
            ("hi", HARDWAREINPUT),
        ]

    class INPUT(ctypes.Structure):
        _anonymous_ = ("_u",)
        _fields_ = [
            ("type", wintypes.DWORD),
            ("_u", INPUT_UNION),
        ]

    ctypes.windll.user32.SendInput.restype = wintypes.UINT
    ctypes.windll.user32.SendInput.argtypes = [wintypes.UINT, ctypes.c_void_p, ctypes.c_int]

    utf16_bytes = text.encode("utf-16le")
    code_units = [
        int.from_bytes(utf16_bytes[i : i + 2], "little")
        for i in range(0, len(utf16_bytes), 2)
    ]

    inputs = []
    for code in code_units:
        inp_down = INPUT(type=INPUT_KEYBOARD)
        inp_down.ki = KEYBDINPUT(
            wVk=0,
            wScan=code,
            dwFlags=KEYEVENTF_UNICODE,
            time=0,
            dwExtraInfo=0,
        )
        inputs.append(inp_down)

        inp_up = INPUT(type=INPUT_KEYBOARD)
        inp_up.ki = KEYBDINPUT(
            wVk=0,
            wScan=code,
            dwFlags=KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
            time=0,
            dwExtraInfo=0,
        )
        inputs.append(inp_up)

    if inputs:
        arr = (INPUT * len(inputs))(*inputs)
        sent = ctypes.windll.user32.SendInput(len(inputs), arr, ctypes.sizeof(INPUT))
        if sent != len(inputs):
            raise RuntimeError(f"SendInput injected {sent}/{len(inputs)} input events.")

    return "sendinput"


def setup_windows_keyboard_hook(callback, log_session_event) -> None:
    """Install the Win+Z low-level hook and keep blocking work off the hook thread."""
    global _GLOBAL_HOOK_CB, _GLOBAL_HOOK_HANDLE, _GLOBAL_HOOK_THREAD
    global _GLOBAL_TOGGLE_QUEUE, _GLOBAL_CONTROL_WORKER_THREAD

    user32 = ctypes.windll.user32
    kernel32 = ctypes.windll.kernel32

    WH_KEYBOARD_LL = 13
    WM_KEYDOWN = 0x0100
    WM_KEYUP = 0x0101
    WM_SYSKEYDOWN = 0x0104
    WM_SYSKEYUP = 0x0105
    VK_LWIN = 0x5B
    VK_RWIN = 0x5C
    VK_Z = 0x5A
    VK_NONAME = 0xFC
    LLKHF_INJECTED = 0x00000010
    KEYEVENTF_KEYUP = 0x0002
    INPUT_KEYBOARD = 1
    ULONG_PTR = ctypes.c_ulonglong if ctypes.sizeof(ctypes.c_void_p) == 8 else wintypes.DWORD

    class KBDLLHOOKSTRUCT(ctypes.Structure):
        _fields_ = [
            ("vkCode", wintypes.DWORD),
            ("scanCode", wintypes.DWORD),
            ("flags", wintypes.DWORD),
            ("time", wintypes.DWORD),
            ("dwExtraInfo", ULONG_PTR),
        ]

    class KEYBDINPUT(ctypes.Structure):
        _fields_ = [
            ("wVk", wintypes.WORD),
            ("wScan", wintypes.WORD),
            ("dwFlags", wintypes.DWORD),
            ("time", wintypes.DWORD),
            ("dwExtraInfo", ULONG_PTR),
        ]

    class HARDWAREINPUT(ctypes.Structure):
        _fields_ = [
            ("uMsg", wintypes.DWORD),
            ("wParamL", wintypes.WORD),
            ("wParamH", wintypes.WORD),
        ]

    class MOUSEINPUT(ctypes.Structure):
        _fields_ = [
            ("dx", wintypes.LONG),
            ("dy", wintypes.LONG),
            ("mouseData", wintypes.DWORD),
            ("dwFlags", wintypes.DWORD),
            ("time", wintypes.DWORD),
            ("dwExtraInfo", ULONG_PTR),
        ]

    class INPUT_UNION(ctypes.Union):
        _fields_ = [
            ("mi", MOUSEINPUT),
            ("ki", KEYBDINPUT),
            ("hi", HARDWAREINPUT),
        ]

    class INPUT(ctypes.Structure):
        _anonymous_ = ("_u",)
        _fields_ = [
            ("type", wintypes.DWORD),
            ("_u", INPUT_UNION),
        ]

    HOOKPROC = ctypes.WINFUNCTYPE(
        ctypes.c_longlong,
        ctypes.c_int,
        wintypes.WPARAM,
        wintypes.LPARAM,
    )

    user32.SetWindowsHookExW.restype = wintypes.HHOOK
    user32.SetWindowsHookExW.argtypes = [
        ctypes.c_int,
        HOOKPROC,
        wintypes.HINSTANCE,
        wintypes.DWORD,
    ]
    user32.UnhookWindowsHookEx.restype = wintypes.BOOL
    user32.UnhookWindowsHookEx.argtypes = [wintypes.HHOOK]
    user32.CallNextHookEx.restype = ctypes.c_longlong
    user32.CallNextHookEx.argtypes = [
        wintypes.HHOOK,
        ctypes.c_int,
        wintypes.WPARAM,
        wintypes.LPARAM,
    ]
    user32.GetAsyncKeyState.restype = wintypes.SHORT
    user32.GetAsyncKeyState.argtypes = [ctypes.c_int]
    user32.GetKeyState.restype = wintypes.SHORT
    user32.GetKeyState.argtypes = [ctypes.c_int]
    user32.SendInput.restype = wintypes.UINT
    user32.SendInput.argtypes = [wintypes.UINT, ctypes.c_void_p, ctypes.c_int]

    def send_dummy_key() -> None:
        """Mark Win as part of a chord without using deprecated keybd_event."""
        events = (INPUT * 2)()
        events[0].type = INPUT_KEYBOARD
        events[0].ki = KEYBDINPUT(
            wVk=VK_NONAME,
            wScan=0,
            dwFlags=0,
            time=0,
            dwExtraInfo=0,
        )
        events[1].type = INPUT_KEYBOARD
        events[1].ki = KEYBDINPUT(
            wVk=VK_NONAME,
            wScan=0,
            dwFlags=KEYEVENTF_KEYUP,
            time=0,
            dwExtraInfo=0,
        )
        user32.SendInput(2, events, ctypes.sizeof(INPUT))

    _GLOBAL_TOGGLE_QUEUE = queue.SimpleQueue()

    def control_worker():
        assert _GLOBAL_TOGGLE_QUEUE is not None
        while True:
            evt = _GLOBAL_TOGGLE_QUEUE.get()
            if evt == "toggle":
                log_session_event("Hotkey Win+Z received; invoking toggle.")
                try:
                    callback()
                except Exception as exc:
                    log_session_event(
                        f"Toggle callback failed: {type(exc).__name__}: {exc}"
                    )
                    print(f"[ERROR] Toggle handler failed: {exc}", file=sys.stderr)

    _GLOBAL_CONTROL_WORKER_THREAD = threading.Thread(
        target=control_worker,
        daemon=True,
    )
    _GLOBAL_CONTROL_WORKER_THREAD.start()

    win_state = False
    last_enqueue_time = 0.0

    def hook_callback(nCode: int, wParam: int, lParam: int) -> int:
        nonlocal win_state, last_enqueue_time

        if nCode >= 0:
            kbd = KBDLLHOOKSTRUCT.from_address(lParam)

            # Ignore our own SendInput events and text injection events.
            if kbd.flags & LLKHF_INJECTED:
                return user32.CallNextHookEx(None, nCode, wParam, lParam)

            vk = kbd.vkCode
            is_down = wParam in (WM_KEYDOWN, WM_SYSKEYDOWN)
            is_up = wParam in (WM_KEYUP, WM_SYSKEYUP)

            if vk in (VK_LWIN, VK_RWIN):
                if is_down:
                    win_state = True
                elif is_up:
                    win_state = False
            elif vk == VK_Z:
                win_active = (
                    win_state
                    or bool(user32.GetAsyncKeyState(VK_LWIN) & 0x8000)
                    or bool(user32.GetAsyncKeyState(VK_RWIN) & 0x8000)
                    or bool(user32.GetKeyState(VK_LWIN) & 0x8000)
                    or bool(user32.GetKeyState(VK_RWIN) & 0x8000)
                )
                if win_active:
                    if is_down:
                        # A harmless VK_NONAME chord prevents the Start menu
                        # from treating the later Win release as a bare Win tap.
                        send_dummy_key()

                        now = time.monotonic()
                        if now - last_enqueue_time > 0.35:
                            last_enqueue_time = now
                            assert _GLOBAL_TOGGLE_QUEUE is not None
                            _GLOBAL_TOGGLE_QUEUE.put("toggle")
                        return 1
                    if is_up:
                        return 1

        return user32.CallNextHookEx(None, nCode, wParam, lParam)

    _GLOBAL_HOOK_CB = HOOKPROC(hook_callback)

    class MSG(ctypes.Structure):
        _fields_ = [
            ("hwnd", wintypes.HWND),
            ("message", wintypes.UINT),
            ("wParam", wintypes.WPARAM),
            ("lParam", wintypes.LPARAM),
            ("time", wintypes.DWORD),
            ("pt", wintypes.POINT),
        ]

    ready_evt = threading.Event()
    hook_error: list[int] = []

    def hook_worker():
        global _GLOBAL_HOOK_HANDLE

        _GLOBAL_HOOK_HANDLE = user32.SetWindowsHookExW(
            WH_KEYBOARD_LL,
            _GLOBAL_HOOK_CB,
            None,
            0,
        )
        if not _GLOBAL_HOOK_HANDLE:
            hook_error.append(int(kernel32.GetLastError()))
            ready_evt.set()
            return

        ready_evt.set()
        msg = MSG()
        while user32.GetMessageW(ctypes.byref(msg), None, 0, 0) > 0:
            user32.TranslateMessage(ctypes.byref(msg))
            user32.DispatchMessageW(ctypes.byref(msg))

        if _GLOBAL_HOOK_HANDLE:
            user32.UnhookWindowsHookEx(_GLOBAL_HOOK_HANDLE)
            _GLOBAL_HOOK_HANDLE = None

    _GLOBAL_HOOK_THREAD = threading.Thread(target=hook_worker, daemon=True)
    _GLOBAL_HOOK_THREAD.start()

    if not ready_evt.wait(timeout=2.0):
        raise RuntimeError("Timed out while installing the Windows keyboard hook.")
    if not _GLOBAL_HOOK_HANDLE:
        code = hook_error[0] if hook_error else 0
        raise RuntimeError(f"Could not install the Windows keyboard hook (Win32 error {code}).")


class WindowsOSD:
    """Non-intrusive floating OSD capsule at bottom-right corner."""

    def __init__(self):
        self._cmd_queue: queue.Queue = queue.Queue()
        self._ready = threading.Event()
        self._error = None
        self._thread = threading.Thread(target=self._ui_loop, daemon=True)
        self._thread.start()
        if not self._ready.wait(3):
            raise RuntimeError("Recording overlay did not start.")
        if self._error:
            raise RuntimeError(f"Recording overlay failed: {self._error}")

    def show(self, text: str, dot_color: str, auto_hide_ms: int | None = None) -> None:
        if self._error or not self._thread.is_alive():
            raise RuntimeError(f"Recording overlay is unavailable: {self._error}")
        self._cmd_queue.put(("show", text, dot_color, auto_hide_ms))

    def hide(self) -> None:
        self._cmd_queue.put(("hide",))

    def _ui_loop(self) -> None:
        try:
            import tkinter as tk
        except ImportError as exc:
            self._error = exc
            self._ready.set()
            return

        try:
            root = tk.Tk()
            root.overrideredirect(True)
            root.attributes("-topmost", True)
            root.attributes("-alpha", 0.92)
            root.withdraw()

            bg_color = "#181825"
            root.configure(bg=bg_color)

            frame = tk.Frame(
                root,
                bg=bg_color,
                padx=16,
                pady=10,
                highlightbackground="#313244",
                highlightthickness=1,
            )
            frame.pack()

            dot_label = tk.Label(
                frame,
                text="●",
                font=("Segoe UI", 15, "bold"),
                fg="#f38ba8",
                bg=bg_color,
            )
            dot_label.pack(side=tk.LEFT, padx=(0, 8))

            text_label = tk.Label(
                frame,
                text="",
                font=("Segoe UI", 10, "bold"),
                fg="#cdd6f4",
                bg=bg_color,
            )
            text_label.pack(side=tk.LEFT)

            def apply_noactivate():
                try:
                    hwnd = ctypes.windll.user32.GetParent(root.winfo_id())
                    if not hwnd:
                        hwnd = root.winfo_id()
                    GWL_EXSTYLE = -20
                    WS_EX_NOACTIVATE = 0x08000000
                    WS_EX_TOOLWINDOW = 0x00000080
                    WS_EX_TOPMOST = 0x00000008
                    current_style = ctypes.windll.user32.GetWindowLongW(hwnd, GWL_EXSTYLE)
                    ctypes.windll.user32.SetWindowLongW(
                        hwnd,
                        GWL_EXSTYLE,
                        current_style | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                    )
                except Exception:
                    pass

            auto_hide_timer = None

            def poll_queue():
                nonlocal auto_hide_timer
                try:
                    while True:
                        cmd = self._cmd_queue.get_nowait()
                        if cmd[0] == "show":
                            _, text, color, auto_hide_ms = cmd
                            if auto_hide_timer is not None:
                                root.after_cancel(auto_hide_timer)
                                auto_hide_timer = None
                            dot_label.config(fg=color)
                            text_label.config(text=text)
                            root.deiconify()
                            root.update_idletasks()
                            apply_noactivate()

                            w = root.winfo_reqwidth()
                            h = root.winfo_reqheight()
                            sw = root.winfo_screenwidth()
                            sh = root.winfo_screenheight()
                            x = sw - w - 35
                            y = sh - h - 60
                            root.geometry(f"+{x}+{y}")

                            if auto_hide_ms is not None:
                                auto_hide_timer = root.after(auto_hide_ms, root.withdraw)
                        elif cmd[0] == "hide":
                            if auto_hide_timer is not None:
                                root.after_cancel(auto_hide_timer)
                                auto_hide_timer = None
                            root.withdraw()
                except queue.Empty:
                    pass
                root.after(50, poll_queue)

            root.after(50, poll_queue)
            self._ready.set()
            root.mainloop()
        except Exception as exc:
            self._error = exc
            self._ready.set()


