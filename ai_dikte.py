#!/usr/bin/env python3
"""Single application entrypoint for the source install and frozen executable."""
from __future__ import annotations
import os
import sys
from pathlib import Path

VERSION = "0.4.0"
_DAEMON_MUTEX_HANDLE = None

def init_windows_console() -> None:
    if sys.platform != "win32":
        return
    try:
        import ctypes

        ATTACH_PARENT_PROCESS = -1
        kernel32 = ctypes.windll.kernel32
        kernel32.AttachConsole.restype = ctypes.c_int
        kernel32.AttachConsole.argtypes = [ctypes.c_int]
        kernel32.AttachConsole(ATTACH_PARENT_PROCESS)
        if sys.stdout is None or getattr(sys.stdout, "closed", False):
            sys.stdout = open("CONOUT$", "w", encoding="utf-8", errors="replace")
        elif hasattr(sys.stdout, "reconfigure"):
            try:
                sys.stdout.reconfigure(encoding="utf-8", errors="replace")
            except Exception:
                pass
        if sys.stderr is None or getattr(sys.stderr, "closed", False):
            sys.stderr = open("CONOUT$", "w", encoding="utf-8", errors="replace")
        elif hasattr(sys.stderr, "reconfigure"):
            try:
                sys.stderr.reconfigure(encoding="utf-8", errors="replace")
            except Exception:
                pass
    except Exception:
        pass

def acquire_windows_daemon_mutex(command: str | None) -> bool:
    """Ensure only one background hotkey daemon runs in a Windows user session."""
    global _DAEMON_MUTEX_HANDLE

    if sys.platform != "win32" or command not in {"daemon", "run"}:
        return True

    import ctypes

    ERROR_ALREADY_EXISTS = 183
    kernel32 = ctypes.windll.kernel32
    kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
    kernel32.CreateMutexW.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_wchar_p]
    kernel32.CreateMutexW.restype = ctypes.c_void_p
    handle = kernel32.CreateMutexW(None, False, "Local\\AI-Dikte-Daemon")
    if not handle:
        print("[ERROR] Could not create Windows daemon mutex.", file=sys.stderr)
        return False

    if kernel32.GetLastError() == ERROR_ALREADY_EXISTS:
        kernel32.CloseHandle(handle)
        print("[INFO] AI Dikte daemon is already running.")
        return False

    _DAEMON_MUTEX_HANDLE = handle
    return True


def runtime_self_test() -> int:
    import ai_dikte_core as core
    from ai_dikte_ui import show_dialog
    import websockets
    from tkinter import ttk
    if not callable(websockets.connect) or not callable(show_dialog) or not ttk.Frame:
        raise RuntimeError("Required runtime modules are missing.")
    if sys.platform == "win32":
        import sounddevice
        import pystray
        from PIL import Image
        import ai_dikte_win32
        if not sounddevice.RawInputStream or not pystray.Icon:
            raise RuntimeError("Required Windows backend is missing.")
        Image.new("RGBA", (2, 2))
        if ai_dikte_win32.output_text_windows("") != "sendinput":
            raise RuntimeError("SendInput self-test failed.")
    segments = []
    for text in ("Evet.", "Evet."):
        core.merge_final_segment(segments, text)
    if segments != ["Evet.", "Evet."]:
        raise RuntimeError("Final transcript segments were lost.")
    print(f"AI Dikte {VERSION}: runtime self-test passed")
    return 0


def main() -> None:
    init_windows_console()
    import ai_dikte_core as core
    command = sys.argv[1] if len(sys.argv) > 1 else None
    if command in ("--version", "version"):
        print(f"AI Dikte {VERSION}")
        return
    if command in ("--self-test", "self-test"):
        raise SystemExit(runtime_self_test())
    if command is None:
        if sys.platform == "win32":
            if not core.load_api_key(required=False):
                core.setup()
            command = "daemon"
        else:
            command = "setup"
        sys.argv.append(command)
    if not acquire_windows_daemon_mutex(command):
        return
    core.main()


def entrypoint() -> None:
    try:
        main()
    except KeyboardInterrupt:
        raise SystemExit(130)
    except Exception as exc:
        # A windowed EXE has no console: fatal errors must still be visible.
        message = f"AI Dikte failed: {exc}"
        print(message, file=sys.stderr)
        if sys.platform == "win32":
            import ctypes
            ctypes.windll.user32.MessageBoxW(None, message, "AI Dikte — Error", 0x10)
        raise SystemExit(1)


if __name__ == "__main__":
    entrypoint()
