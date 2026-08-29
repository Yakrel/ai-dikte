#!/usr/bin/env python3
"""Windows & cross-platform entrypoint for AI Dikte."""

from __future__ import annotations

# Keep imports used by the bundled `ai-dikte` data script visible to PyInstaller.
# The shared runtime is executed with runpy, so PyInstaller cannot discover its
# imports by statically analysing that file when it is added via --add-data.
import asyncio
import base64
import getpass
import json
import os
import queue
import runpy
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import traceback
from pathlib import Path
from urllib.parse import quote

# These imports are also performed dynamically by the shared Windows runtime.
# Importing them here ensures the frozen build contains the corresponding
# standard-library modules and Tk runtime used by the OSD.
if sys.platform == "win32":
    import ctypes
    import msvcrt
    import tkinter  # noqa: F401
    import winsound  # noqa: F401
    from ctypes import wintypes  # noqa: F401


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
        if sys.stderr is None or getattr(sys.stderr, "closed", False):
            sys.stderr = open("CONOUT$", "w", encoding="utf-8", errors="replace")
    except Exception:
        pass


init_windows_console()

VERSION = "0.1.0"
_DAEMON_MUTEX_HANDLE = None


def bundled_script_path() -> Path:
    """Return the real ai-dikte script in source and PyInstaller builds."""
    if getattr(sys, "frozen", False):
        exe_dir = Path(sys.executable).resolve().parent
        override = exe_dir / "ai-dikte"
        if override.is_file():
            return override
        bundle_dir = Path(getattr(sys, "_MEIPASS", exe_dir))
        return bundle_dir / "ai-dikte"
    return Path(__file__).resolve().parent / "ai-dikte"


def normalize_frozen_child_argv() -> None:
    """Fix child commands spawned by the source script when running frozen."""
    if not getattr(sys, "frozen", False) or len(sys.argv) < 3:
        return

    candidate = Path(sys.argv[1])
    if candidate.name in {"ai-dikte", "ai_dikte.py"} or str(candidate).endswith("ai-dikte"):
        sys.argv = [sys.argv[0], *sys.argv[2:]]


def acquire_windows_daemon_mutex(command: str | None) -> bool:
    """Ensure only one background hotkey daemon runs in a Windows user session."""
    global _DAEMON_MUTEX_HANDLE

    if sys.platform != "win32" or command not in {"daemon", "run"}:
        return True

    import ctypes

    ERROR_ALREADY_EXISTS = 183
    kernel32 = ctypes.windll.kernel32
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


def start_windows_daemon_background(script_path: Path) -> None:
    """Start the daemon without creating a visible console window."""
    if sys.platform != "win32":
        return

    CREATE_NO_WINDOW = 0x08000000
    DETACHED_PROCESS = 0x00000008

    if getattr(sys, "frozen", False):
        cmd = [sys.executable, "daemon"]
    else:
        cmd = [sys.executable, str(script_path), "daemon"]

    subprocess.Popen(
        cmd,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        creationflags=CREATE_NO_WINDOW | DETACHED_PROCESS,
        close_fds=False,
    )


def windows_first_run_setup(script_path: Path) -> bool:
    """Make a directly downloaded Windows EXE useful on first double-click."""
    if sys.platform != "win32" or len(sys.argv) != 1:
        return False

    namespace = runpy.run_path(str(script_path), run_name="_ai_dikte_first_run")
    config = namespace["load_config"](required=False)
    if str(config.get("api_key", "")).strip():
        return False

    print("AI Dikte first-time setup")
    print("Enter your Google AI API key. The Windows dictation hotkey is Win+Z.")
    namespace["setup"]()
    namespace["doctor"]()
    start_windows_daemon_background(script_path)
    print("[OK] AI Dikte is configured and the background hotkey listener has started.")
    print("You can close this window and use Win+Z.")
    return True


def runtime_self_test() -> int:
    """Verify the frozen Windows build contains its runtime dependencies."""
    failures: list[str] = []

    try:
        import websockets

        if not callable(getattr(websockets, "connect", None)):
            failures.append("websockets.connect unavailable")
    except Exception as exc:  # pragma: no cover - exercised in frozen CI
        failures.append(f"websockets: {exc}")

    try:
        import sounddevice

        if not hasattr(sounddevice, "RawInputStream"):
            failures.append("sounddevice.RawInputStream unavailable")
    except Exception as exc:  # pragma: no cover - exercised in frozen CI
        failures.append(f"sounddevice: {exc}")

    try:
        import pystray

        if not hasattr(pystray, "Icon"):
            failures.append("pystray.Icon unavailable")
    except Exception as exc:  # pragma: no cover - exercised in frozen CI
        failures.append(f"pystray: {exc}")

    try:
        from PIL import Image, ImageDraw

        image = Image.new("RGBA", (2, 2), (0, 0, 0, 0))
        ImageDraw.Draw(image).point((0, 0), fill="white")
    except Exception as exc:  # pragma: no cover - exercised in frozen CI
        failures.append(f"Pillow: {exc}")

    script_path = bundled_script_path()
    if not script_path.is_file():
        failures.append(f"bundled script missing: {script_path}")
    else:
        try:
            namespace = runpy.run_path(str(script_path), run_name="_ai_dikte_runtime_self_test")
            if sys.platform == "win32":
                driver = namespace["available_output_driver"]({})
                if driver != "sendinput":
                    failures.append(f"Windows output driver mismatch: {driver}")
                if namespace["output_text_windows"]("") != "sendinput":
                    failures.append("SendInput backend self-test failed")
                if namespace["config_hotkey"]({}) != "win+z":
                    failures.append("Windows hotkey must be fixed to win+z")

            merge_final_segment = namespace.get("merge_final_segment")
            if not callable(merge_final_segment):
                failures.append("transcript merge helper unavailable")
            else:
                segments: list[str] = []
                merge_final_segment(segments, "Birinci cümle.")
                merge_final_segment(segments, "İkinci cümle.")
                if segments != ["Birinci cümle.", "İkinci cümle."]:
                    failures.append("final transcript segments are not preserved")
        except Exception as exc:  # pragma: no cover - exercised in frozen CI
            failures.append(f"shared runtime: {exc}")

    if failures:
        for failure in failures:
            print(f"[FAIL] {failure}", file=sys.stderr)
        return 1

    mode = "frozen" if getattr(sys, "frozen", False) else "source"
    print(f"AI Dikte runtime self-test OK ({mode})")
    return 0


def main() -> None:
    normalize_frozen_child_argv()
    command = sys.argv[1] if len(sys.argv) > 1 else None

    if command in {"--version", "version"}:
        print(f"AI Dikte {VERSION}")
        raise SystemExit(0)

    if command in {"--self-test", "self-test"}:
        raise SystemExit(runtime_self_test())

    script_path = bundled_script_path()
    if not script_path.is_file():
        print(f"[ERROR] Could not find bundled ai-dikte script: {script_path}", file=sys.stderr)
        raise SystemExit(1)

    if windows_first_run_setup(script_path):
        return

    command = sys.argv[1] if len(sys.argv) > 1 else None
    if not acquire_windows_daemon_mutex(command):
        return

    runpy.run_path(str(script_path), run_name="__main__")


if __name__ == "__main__":
    main()
