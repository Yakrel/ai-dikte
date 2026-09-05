#!/usr/bin/env python3
"""Cross-platform minimal dictation tool using Gemini 3.5 Transcribe Live.

Supports Linux (Wayland: Hyprland/KDE) and Windows (Direct SendInput & SoundDevice).
"""

from __future__ import annotations

import asyncio
import base64
import json
import os
import queue
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import traceback
from pathlib import Path
from typing import Any
from ai_dikte_config import (
    APP, CONFIG_DIR, CONFIG_FILE,
    load_config, write_config, load_api_key, config_language, config_mode, config_vocabulary, config_output_driver, config_hotkey, config_audio_cue, config_notify_mode, config_input_device, build_setup_config, save_setup_config,
)
from urllib.parse import quote

# Platform detection
IS_WINDOWS = sys.platform == "win32"

if IS_WINDOWS:
    for stream_name in ("stdout", "stderr"):
        stream = getattr(sys, stream_name, None)
        if stream and hasattr(stream, "reconfigure"):
            try:
                stream.reconfigure(encoding="utf-8", errors="replace")
            except Exception:
                pass

try:
    import websockets
except ImportError:  # pragma: no cover
    websockets = None

try:
    import sounddevice  # type: ignore
except ImportError:  # pragma: no cover
    sounddevice = None

if IS_WINDOWS:
    import ctypes
    import msvcrt
    from ctypes import wintypes
    from ai_dikte_win32 import output_text_windows, setup_windows_keyboard_hook, WindowsOSD
else:
    import fcntl

MODEL = "gemini-3.5-transcribe-live"
AUDIO_RATE = 16000
AUDIO_CHANNELS = 1
SD_AUDIO_FORMAT = "int16"  # sounddevice PortAudio format
PW_AUDIO_FORMAT = "s16"  # pw-record PipeWire format
AUDIO_CHUNK_BYTES = 3200  # 100 ms: 1600 frames * 2 bytes (16-bit PCM)
WS_ENDPOINT = (
    "wss://generativelanguage.googleapis.com/ws/"
    "google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent"
)

# Paths configuration
if IS_WINDOWS:
    RUNTIME_DIR = Path(os.environ.get("LOCALAPPDATA", Path(tempfile.gettempdir()))) / APP / "runtime"
else:
    RUNTIME_DIR = Path(os.environ.get("XDG_RUNTIME_DIR", f"/tmp/{APP}-{os.getuid()}")) / APP

PID_FILE = RUNTIME_DIR / "session.pid"
READY_FILE = RUNTIME_DIR / "session.ready"
STOP_FILE = RUNTIME_DIR / "session.stop"
ERROR_FILE = RUNTIME_DIR / "recorder.stderr"
SESSION_LOG = RUNTIME_DIR / "session.log"
LOCK_FILE = RUNTIME_DIR / "command.lock"

# Linux Hyprland shortcut definitions
HYPR_CONFIG_DIR = Path.home() / ".config" / "hypr"
HYPR_BINDINGS_LUA = HYPR_CONFIG_DIR / "bindings.lua"
HYPR_BINDINGS_CONF = HYPR_CONFIG_DIR / "bindings.conf"
HYPR_HYPRLAND_CONF = HYPR_CONFIG_DIR / "hyprland.conf"
HYPRLAND_LUA = HYPR_CONFIG_DIR / "hyprland.lua"
HYPR_MARKER_START_LUA = "-- >>> ai-dikte >>>"
HYPR_MARKER_END_LUA = "-- <<< ai-dikte <<<"
HYPR_MARKER_START_CONF = "# >>> ai-dikte >>>"
HYPR_MARKER_END_CONF = "# <<< ai-dikte <<<"
HYPR_BINDING_LUA = 'o.bind("SUPER + Z", "AI Dikte", "/usr/bin/ai-dikte-toggle")'
HYPR_BINDING_CONF = "bind = SUPER, Z, exec, /usr/bin/ai-dikte-toggle"

_GLOBAL_TRAY_ICON: Any = None
_GLOBAL_OSD: Any = None  # WindowsOSD, initialized lazily


def ensure_runtime() -> None:
    RUNTIME_DIR.mkdir(parents=True, exist_ok=True)
    if not IS_WINDOWS:
        try:
            RUNTIME_DIR.chmod(0o700)
        except OSError:
            pass

def log_session_event(msg: str) -> None:
    """Append non-sensitive diagnostic metadata to session.log."""
    try:
        RUNTIME_DIR.mkdir(parents=True, exist_ok=True)
        ts = time.strftime("%Y-%m-%d %H:%M:%S")
        with SESSION_LOG.open("a", encoding="utf-8") as f:
            f.write(f"[{ts}] {msg}\n")
    except Exception:
        pass


class FileLock:
    """Cross-platform non-blocking file lock."""

    def __init__(self, path: Path):
        self.path = path
        self.file_handle = None

    def __enter__(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        if IS_WINDOWS:
            self.file_handle = open(self.path, "w")
            try:
                msvcrt.locking(self.file_handle.fileno(), msvcrt.LK_NBLCK, 1)
            except OSError:
                self.file_handle.close()
                fail("Previous command is still running.")
        else:
            self.file_handle = open(self.path, "a+")
            try:
                fcntl.flock(self.file_handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            except (BlockingIOError, OSError):
                self.file_handle.close()
                fail("Previous command is still running.")
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.file_handle:
            try:
                if IS_WINDOWS:
                    self.file_handle.seek(0)
                    msvcrt.locking(self.file_handle.fileno(), msvcrt.LK_UNLCK, 1)
                else:
                    fcntl.flock(self.file_handle.fileno(), fcntl.LOCK_UN)
            except OSError:
                pass
            self.file_handle.close()


def play_audio_cue(cue: str) -> None:
    if not IS_WINDOWS or not config_audio_cue(load_config()):
        return
    import winsound
    tones = {"start": [(880, 50)], "stop": [(587, 50)],
             "finish": [(880, 40), (1174, 50)], "error": [(440, 100)]}
    for frequency, duration in tones[cue]:
        winsound.Beep(frequency, duration)


def get_windows_osd():
    global _GLOBAL_OSD
    if _GLOBAL_OSD is None:
        _GLOBAL_OSD = WindowsOSD()
    return _GLOBAL_OSD


def notify(
    title: str,
    message: str,
    urgency: str = "normal",
    event_type: str = "info",
) -> None:
    config = load_config(required=False)
    notify_mode = config_notify_mode(config)

    if urgency != "critical" and event_type != "error" and notify_mode == "none":
        return

    if IS_WINDOWS:
        osd = get_windows_osd()
        if osd is not None:
            if event_type == "start":
                osd.show("Listening...", "#f38ba8")
            elif event_type == "transcribing":
                osd.show("Transcribing...", "#f9e2af")
            elif event_type == "finish":
                osd.show("Text inserted", "#a6e3a1", auto_hide_ms=1200)
            elif event_type == "error":
                osd.show(f"Error: {message[:35]}", "#f38ba8", auto_hide_ms=3000)
            else:
                osd.show(message, "#89b4fa", auto_hide_ms=2000)
            return

    if not shutil.which("notify-send"):
        raise RuntimeError("Required notification tool is missing: notify-send")
    subprocess.run(["notify-send", "-a", "AI Dikte", "-u", urgency, title, message],
                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=True)


def fail(message: str, code: int = 1) -> None:
    raise RuntimeError(message)


def list_input_devices() -> list[tuple[int | None, str]]:
    devices: list[tuple[int | None, str]] = [(None, "System default")]
    if sounddevice is None:
        raise RuntimeError("Required recorder is missing: sounddevice")
    for index, device in enumerate(sounddevice.query_devices()):
        if int(device.get("max_input_channels", 0)) > 0:
            devices.append((index, f"{index}: {str(device.get('name', 'Microphone')).strip()}"))
    return devices


def microphone_label(config: dict[str, Any]) -> str:
    selected = config_input_device(config)
    if selected is None:
        return 'System default'
    try:
        device = sounddevice.query_devices(selected, "input") if sounddevice else None
        if device:
            return f"{selected}: {str(device.get('name', 'Microphone')).strip()}"
    except Exception:
        pass
    return f"{selected}: {'Unavailable'}"

def microphone_available(config: dict[str, Any]) -> bool:
    if sounddevice is None:
        return False
    try:
        sounddevice.check_input_settings(
            device=config_input_device(config),
            channels=AUDIO_CHANNELS,
            dtype=SD_AUDIO_FORMAT,
            samplerate=AUDIO_RATE,
        )
        return True
    except Exception:
        return False


def save_settings(key: str, candidate: dict[str, Any]) -> None:
    """Validate API-affecting changes; save local preferences without a network call."""
    key = key.strip()
    if not key:
        raise RuntimeError("API key cannot be empty.")
    existing = load_config(required=False)
    old_key = load_api_key(required=False)
    candidate = build_setup_config(candidate)
    normalized_existing = build_setup_config(existing)
    api_fields = ("language", "mode", "custom_vocabulary")
    if not existing or key != old_key or any(
        candidate.get(field) != normalized_existing.get(field) for field in api_fields
    ):
        validate_api_key(key, candidate)
    save_setup_config(key, candidate)
    if not IS_WINDOWS and desktop_kind() == "hyprland":
        install_hyprland_shortcut()


def setup() -> None:
    if not run_tray_gui_command("setup"):
        raise SystemExit(1)


def desktop_kind() -> str:
    if IS_WINDOWS:
        return "windows"
    desktop = " ".join(
        filter(
            None,
            [
                os.environ.get("XDG_CURRENT_DESKTOP", ""),
                os.environ.get("XDG_SESSION_DESKTOP", ""),
                os.environ.get("DESKTOP_SESSION", ""),
            ],
        )
    ).lower()

    if os.environ.get("HYPRLAND_INSTANCE_SIGNATURE") or "hyprland" in desktop:
        return "hyprland"
    if os.environ.get("KDE_FULL_SESSION") or "kde" in desktop or "plasma" in desktop:
        return "kde"
    return "other"


def output_candidates(config: dict[str, Any] | None = None) -> list[str]:
    if IS_WINDOWS:
        return ["sendinput"]

    config = config or load_config(required=False)
    forced = config_output_driver(config)
    if forced != "auto":
        return [forced]

    desktop = desktop_kind()
    if desktop == "hyprland":
        return ["wtype"]
    if desktop == "kde":
        return ["kwtype"]
    raise RuntimeError("Unsupported desktop: use KDE Plasma or Hyprland on Wayland.")


def available_output_driver(config: dict[str, Any] | None = None) -> str | None:
    if IS_WINDOWS:
        return "sendinput"
    for driver in output_candidates(config):
        if shutil.which(driver):
            return driver
    return None


def output_text(text: str) -> str:
    if IS_WINDOWS:
        return output_text_windows(text)
    driver = output_candidates()[0]
    executable = shutil.which(driver)
    if not executable:
        raise RuntimeError(f"Required typing backend is missing: {driver}")
    time.sleep(0.06)
    result = subprocess.run([executable, "--", text], stdout=subprocess.DEVNULL,
                            stderr=subprocess.PIPE, text=True, timeout=15)
    if result.returncode:
        raise RuntimeError(f"{driver} failed: {result.stderr.strip() or result.returncode}")
    return driver


def hyprland_shortcut_target() -> tuple[Path, str, str, str]:
    if HYPRLAND_LUA.exists() or HYPR_BINDINGS_LUA.exists():
        return HYPR_BINDINGS_LUA, HYPR_BINDING_LUA, HYPR_MARKER_START_LUA, HYPR_MARKER_END_LUA
    if HYPR_BINDINGS_CONF.exists():
        return HYPR_BINDINGS_CONF, HYPR_BINDING_CONF, HYPR_MARKER_START_CONF, HYPR_MARKER_END_CONF
    if HYPR_HYPRLAND_CONF.exists():
        return HYPR_HYPRLAND_CONF, HYPR_BINDING_CONF, HYPR_MARKER_START_CONF, HYPR_MARKER_END_CONF
    return HYPR_BINDINGS_CONF, HYPR_BINDING_CONF, HYPR_MARKER_START_CONF, HYPR_MARKER_END_CONF


def remove_managed_shortcut_block(path: Path) -> bool:
    if not path.exists():
        return False

    text = path.read_text(encoding="utf-8")
    modified = False

    for start_marker, end_marker in (
        (HYPR_MARKER_START_LUA, HYPR_MARKER_END_LUA),
        (HYPR_MARKER_START_CONF, HYPR_MARKER_END_CONF),
        ("-- >>> gemini-dikte >>>", "-- <<< gemini-dikte <<<"),
        ("# >>> gemini-dikte >>>", "# <<< gemini-dikte <<<"),
    ):
        while True:
            start = text.find(start_marker)
            if start == -1:
                break
            end = text.find(end_marker, start + len(start_marker))
            if end == -1:
                break
            end += len(end_marker)
            if end < len(text) and text[end] == "\n":
                end += 1
            text = text[:start] + text[end:]
            modified = True

    if modified:
        path.write_text(text, encoding="utf-8")
        return True
    return False


def reload_hyprland() -> None:
    if shutil.which("hyprctl"):
        subprocess.run(
            ["hyprctl", "reload"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )


def install_hyprland_shortcut() -> None:
    if IS_WINDOWS:
        print("[INFO] Hyprland shortcuts are only applicable on Linux.")
        return

    target, binding, start_marker, end_marker = hyprland_shortcut_target()
    target.parent.mkdir(parents=True, exist_ok=True)

    for candidate in (HYPR_BINDINGS_LUA, HYPR_BINDINGS_CONF, HYPR_HYPRLAND_CONF):
        if candidate != target:
            remove_managed_shortcut_block(candidate)

    current = target.read_text(encoding="utf-8") if target.exists() else ""
    if (
        start_marker not in current
        and HYPR_MARKER_START_LUA not in current
        and HYPR_MARKER_START_CONF not in current
    ):
        block = f"{start_marker}\n{binding}\n{end_marker}\n"
        separator = "" if not current or current.endswith("\n") else "\n"
        with target.open("a", encoding="utf-8") as handle:
            handle.write(separator + block)
        print(f"[OK] Installed Hyprland shortcut Meta+Z in {target}")
    else:
        print(f"[OK] Hyprland shortcut already installed in {target}")

    reload_hyprland()


def remove_hyprland_shortcut() -> None:
    if IS_WINDOWS:
        return

    removed: list[Path] = []
    for path in (HYPR_BINDINGS_LUA, HYPR_BINDINGS_CONF, HYPR_HYPRLAND_CONF):
        if remove_managed_shortcut_block(path):
            removed.append(path)

    if removed:
        for path in removed:
            print(f"[OK] Removed Hyprland shortcut from {path}")
        reload_hyprland()
    else:
        print("[OK] No managed AI Dikte shortcut found.")


def read_pid() -> int | None:
    try:
        return int(PID_FILE.read_text(encoding="ascii").strip())
    except (OSError, ValueError):
        PID_FILE.unlink(missing_ok=True)
        return None


def pid_alive(pid: int) -> bool:
    if IS_WINDOWS:
        PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
        SYNCHRONIZE = 0x00100000
        handle = ctypes.windll.kernel32.OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE,
            False,
            pid,
        )
        if not handle:
            return False
        exit_code = wintypes.DWORD()
        res = ctypes.windll.kernel32.GetExitCodeProcess(handle, ctypes.byref(exit_code))
        ctypes.windll.kernel32.CloseHandle(handle)
        STILL_ACTIVE = 259
        return bool(res and exit_code.value == STILL_ACTIVE)

    try:
        os.kill(pid, 0)
        return True
    except (ProcessLookupError, PermissionError):
        return False


def session_matches(pid: int) -> bool:
    if not pid_alive(pid):
        return False

    if IS_WINDOWS:
        return True

    try:
        cmdline = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
    except (OSError, PermissionError):
        return False

    args = [os.fsdecode(arg) for arg in cmdline if arg]
    return "_live-session" in args and any(
        Path(arg).name in ("ai-dikte", "gemini-dikte") for arg in args
    )


def clear_runtime_state() -> None:
    PID_FILE.unlink(missing_ok=True)
    READY_FILE.unlink(missing_ok=True)
    STOP_FILE.unlink(missing_ok=True)
    ERROR_FILE.unlink(missing_ok=True)


class WindowsSessionController:
    """Manages in-process dictation sessions on Windows without spawning child processes."""

    def __init__(self):
        self.lock = threading.Lock()
        self.state = "idle"  # idle | starting | recording | transcribing
        self.stop_event: threading.Event | None = None
        self.session_thread: threading.Thread | None = None

    def toggle(self) -> None:
        with self.lock:
            if self.state in ("starting", "recording"):
                if self.stop_event and not self.stop_event.is_set():
                    self.state = "transcribing"
                    self.stop_event.set()
                    play_audio_cue("stop")
                    notify(
                        "AI Dikte",
                        "Transcribing...",
                        event_type="transcribing",
                    )
                return

            if self.state == "transcribing":
                return

            self.state = "starting"
            self.stop_event = threading.Event()
            play_audio_cue("start")
            notify("AI Dikte", "Listening...", event_type="start")
            self.session_thread = threading.Thread(
                target=self._session_worker,
                args=(self.stop_event,),
                daemon=True,
            )
            self.session_thread.start()

    def _session_worker(self, stop_evt: threading.Event) -> None:
        try:
            asyncio.run(
                live_session(
                    stop_checker=stop_evt.is_set,
                    on_ready=self._on_ready,
                )
            )
        except Exception as exc:
            log_session_event(f"Session worker exception: {type(exc).__name__}: {exc}")
            notify("AI Dikte — Error", str(exc), "critical", event_type="error")
            play_audio_cue("error")
            print(f"[ERROR] Live session failed: {exc}", file=sys.stderr)
        finally:
            with self.lock:
                self.state = "idle"
                self.stop_event = None
                self.session_thread = None
            log_session_event("WindowsSessionController state reset to idle.")

    def _on_ready(self) -> None:
        with self.lock:
            if self.state == "starting":
                self.state = "recording"

    def get_state(self) -> str:
        with self.lock:
            return self.state


_WINDOWS_SESSION_CONTROLLER: WindowsSessionController | None = (
    WindowsSessionController() if IS_WINDOWS else None
)


def start_live_session() -> None:
    load_api_key()
    if websockets is None:
        fail("Python websockets module is missing. Install python-websockets / pip install websockets.")

    if not shutil.which("pw-record"):
        fail("Required recorder is missing: pw-record.")
    if not available_output_driver():
        fail("No supported direct typing driver found (kwtype/wtype).")

    clear_runtime_state()
    SESSION_LOG.unlink(missing_ok=True)

    child_cmd = application_command("_live-session")

    with SESSION_LOG.open("ab", buffering=0) as log_handle:
        process = subprocess.Popen(
            child_cmd,
            stdin=subprocess.DEVNULL,
            stdout=log_handle,
            stderr=subprocess.STDOUT,
            start_new_session=True,
            close_fds=True,
        )

    PID_FILE.write_text(str(process.pid), encoding="ascii")
    try:
        PID_FILE.chmod(0o600)
    except OSError:
        pass
    time.sleep(0.08)
    if process.poll() is not None:
        details = SESSION_LOG.read_text(errors="replace")[-700:] if SESSION_LOG.exists() else ""
        clear_runtime_state()
        fail(f"Could not start dictation session. {details.strip()}")


def request_stop() -> None:
    STOP_FILE.touch(exist_ok=True)
    play_audio_cue("stop")
    notify("AI Dikte", "Transcribing…", event_type="transcribing")


def toggle() -> None:
    if IS_WINDOWS and _WINDOWS_SESSION_CONTROLLER is not None:
        _WINDOWS_SESSION_CONTROLLER.toggle()
        return

    # Linux process-based session toggle
    pid = read_pid()
    if pid and session_matches(pid):
        if STOP_FILE.exists():
            notify(
                "AI Dikte",
                "Previous transcription is still finishing",
                event_type="info",
            )
            return
        request_stop()
        return

    clear_runtime_state()
    start_live_session()


class SoundDeviceStreamRecorder:
    """Asynchronous audio capture using sounddevice RawInputStream."""

    def __init__(
        self,
        sample_rate: int = AUDIO_RATE,
        chunk_bytes: int = AUDIO_CHUNK_BYTES,
        device: int | None = None,
    ):
        self.device = device
        self.sample_rate = sample_rate
        self.chunk_bytes = chunk_bytes
        self.block_frames = chunk_bytes // 2  # 16-bit PCM = 2 bytes per frame
        self.queue: asyncio.Queue[bytes] = asyncio.Queue()
        self.stream = None
        self.loop: asyncio.AbstractEventLoop | None = None
        self.running = False

    def _callback(self, indata: bytes, frames: int, time_info: Any, status: Any) -> None:
        if self.running and self.loop and not self.loop.is_closed():
            self.loop.call_soon_threadsafe(self.queue.put_nowait, bytes(indata))

    def start(self, loop: asyncio.AbstractEventLoop) -> None:
        if sounddevice is None:
            raise RuntimeError("sounddevice is not installed.")
        self.loop = loop
        self.running = True
        self.stream = sounddevice.RawInputStream(
            device=self.device,
            samplerate=self.sample_rate,
            channels=AUDIO_CHANNELS,
            dtype=SD_AUDIO_FORMAT,
            blocksize=self.block_frames,
            callback=self._callback,
        )
        self.stream.start()

    async def read(self) -> bytes:
        return await self.queue.get()

    def stop(self) -> None:
        self.running = False
        if self.stream is not None:
            try:
                self.stream.stop()
                self.stream.close()
            except Exception:
                pass
            self.stream = None


async def stop_recorder(process: asyncio.subprocess.Process | None) -> None:
    if process is None or process.returncode is not None:
        return

    if IS_WINDOWS:
        try:
            process.terminate()
            await asyncio.wait_for(process.wait(), timeout=1.0)
            return
        except (OSError, asyncio.TimeoutError):
            pass
        try:
            process.kill()
            await process.wait()
        except OSError:
            pass
        return

    try:
        os.killpg(process.pid, signal.SIGINT)
    except ProcessLookupError:
        return

    try:
        await asyncio.wait_for(process.wait(), timeout=2.0)
        return
    except asyncio.TimeoutError:
        pass

    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return

    try:
        await asyncio.wait_for(process.wait(), timeout=1.0)
        return
    except asyncio.TimeoutError:
        pass

    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    await process.wait()


def parse_server_error(message: dict[str, Any]) -> str | None:
    error = message.get("error")
    if isinstance(error, dict):
        detail = str(error.get("message", "")).strip()
        code = error.get("code")
        if detail:
            return f"Gemini Live API error{f' {code}' if code else ''}: {detail}"
    return None


async def open_live_websocket(api_key: str, config: dict[str, Any]):
    if websockets is None:
        raise RuntimeError("Python websockets module is missing.")

    url = f"{WS_ENDPOINT}?key={quote(api_key, safe='')}"
    log_session_event(f"Connecting to Gemini Live WebSocket: models/{MODEL}")
    try:
        websocket = await websockets.connect(
            url,
            open_timeout=10,
            close_timeout=2,
            ping_interval=20,
            ping_timeout=20,
            max_size=4 * 1024 * 1024,
        )
    except Exception as exc:
        log_session_event(f"Connection failed: {type(exc).__name__}: {exc}")
        raise RuntimeError(f"Could not connect to Gemini Live API: {exc}") from exc

    try:
        input_transcription: dict[str, Any] = {
            "languageCodes": [config_language(config)],
            "mode": config_mode(config),
        }
        vocabulary = config_vocabulary(config)
        if vocabulary:
            input_transcription["customVocabulary"] = vocabulary
    
        setup_message = {
            "setup": {
                "model": f"models/{MODEL}",
                "generationConfig": {"responseModalities": ["TEXT"]},
                "realtimeInputConfig": {
                    "automaticActivityDetection": {"disabled": True},
                },
                "inputAudioTranscription": input_transcription,
            }
        }
        await websocket.send(json.dumps(setup_message))
        log_session_event("Sent Gemini setup message.")
    
        deadline = time.monotonic() + 10.0
        while time.monotonic() < deadline:
            remaining = deadline - time.monotonic()
            try:
                raw = await asyncio.wait_for(websocket.recv(), timeout=max(0.1, remaining))
            except asyncio.TimeoutError:
                break
            except websockets.exceptions.ConnectionClosed as exc:
                code = getattr(exc, "code", None)
                reason = getattr(exc, "reason", "") or str(exc)
                log_session_event(f"WebSocket closed during setup ({code}): {reason}")
                if code in (1008, 4403):
                    raise RuntimeError(
                        f"Gemini API authentication failed (code {code}): invalid API key or quota exceeded."
                    ) from exc
                raise RuntimeError(
                    f"Gemini Live API connection closed during setup ({code}): {reason}"
                ) from exc
    
            if isinstance(raw, bytes):
                raw = raw.decode("utf-8", errors="replace")
            message = json.loads(raw)
            error = parse_server_error(message)
            if error:
                await websocket.close()
                raise RuntimeError(error)
            if "setupComplete" in message:
                log_session_event("Gemini setup complete.")
                return websocket
    
        await websocket.close()
        raise RuntimeError("Gemini Live API setup timed out.")
    except BaseException:
        await websocket.close()
        raise


async def validate_api_key_async(api_key: str, config: dict[str, Any]) -> None:
    websocket = await open_live_websocket(api_key, config)
    await websocket.close()


def validate_api_key(api_key: str, config: dict[str, Any]) -> None:
    """Validate the exact Live API/model contract before replacing a working key."""
    asyncio.run(validate_api_key_async(api_key, config))


async def send_audio(websocket, chunk: bytes) -> None:
    """Send raw 16 kHz PCM using the current Live API realtimeInput.audio field."""
    message = {
        "realtimeInput": {
            "audio": {
                "data": base64.b64encode(chunk).decode("ascii"),
                "mimeType": f"audio/pcm;rate={AUDIO_RATE}",
            }
        }
    }
    await websocket.send(json.dumps(message))


def merge_final_segment(segments: list[str], text: str) -> None:
    """Each final message is a committed segment, even when words repeat."""
    if text.strip():
        segments.append(text.strip())


async def receive_transcriptions(
    websocket,
    transcripts: asyncio.Queue[tuple[str, str]],
    turn_complete_event: asyncio.Event,
) -> None:
    """Drain Live API messages while keeping finalized and interim text separate."""
    try:
        async for raw in websocket:
            if isinstance(raw, bytes):
                raw = raw.decode("utf-8", errors="replace")

            message = json.loads(raw)
            error = parse_server_error(message)
            if error:
                log_session_event(f"Gemini WS error parsed: {error}")
                raise RuntimeError(error)

            content = message.get("serverContent") or {}
            final = content.get("inputTranscription") or {}
            interim = content.get("interimInputTranscription") or {}
            final_text = str(final.get("text", "")).strip()
            interim_text = str(interim.get("text", "")).strip()

            if final_text:
                log_session_event(
                    f"Captured finalized transcription segment ({len(final_text)} chars)."
                )
                await transcripts.put(("final", final_text))
            elif interim_text:
                log_session_event(
                    f"Captured interim transcription update ({len(interim_text)} chars)."
                )
                await transcripts.put(("interim", interim_text))

            if content.get("turnComplete"):
                log_session_event("Received turnComplete signal.")
                turn_complete_event.set()
    except websockets.exceptions.ConnectionClosed as exc:
        code = getattr(exc, "code", None)
        reason = getattr(exc, "reason", "") or str(exc)
        log_session_event(f"WebSocket closed ({code}): {reason}")
        if code in (1008, 4403):
            raise RuntimeError(
                f"Gemini API authentication failed (code {code}): invalid API key or quota exceeded."
            ) from exc
        if code and code != 1000:
            raise RuntimeError(
                f"Gemini WebSocket connection closed unexpectedly ({code}): {reason}"
            ) from exc


async def raise_if_receiver_failed(receiver_task: asyncio.Task[None]) -> None:
    if receiver_task.done():
        await receiver_task


async def collect_final_transcript(
    transcripts: asyncio.Queue[tuple[str, str]],
    receiver_task: asyncio.Task[None],
    turn_complete_event: asyncio.Event,
    *,
    timeout: float = 10.0,
) -> str:
    """Wait for completion and final text instead of guessing from speech pauses.

    Input transcription is independent of turn messages, so allow a short drain
    after completion and keep waiting if the latest hypothesis is still interim.
    Never silently inject a known-incomplete transcript on timeout/disconnect.
    """
    final_segments: list[str] = []
    pending_interim = False
    deadline = time.monotonic() + timeout
    complete_since: float | None = None
    last_update = time.monotonic()
    while True:
        await raise_if_receiver_failed(receiver_task)
        now = time.monotonic()
        if turn_complete_event.is_set() and complete_since is None:
            complete_since = now
        if (complete_since is not None and transcripts.empty() and not pending_interim
                and now - max(complete_since, last_update) >= 0.5):
            break
        if now >= deadline:
            raise RuntimeError("Timed out waiting for the final transcription. Please try again.")
        if receiver_task.done() and transcripts.empty():
            if turn_complete_event.is_set() and not pending_interim:
                break
            raise RuntimeError("Connection ended before transcription was complete.")
        try:
            kind, text = await asyncio.wait_for(
                transcripts.get(), timeout=min(0.1, deadline - now)
            )
        except asyncio.TimeoutError:
            continue
        last_update = time.monotonic()
        if kind == "final":
            merge_final_segment(final_segments, text)
            pending_interim = False
        elif kind == "interim":
            pending_interim = bool(text.strip())
    result = " ".join(final_segments).strip()
    if not result:
        raise RuntimeError("Gemini returned no transcription.")
    log_session_event(f"Collected {len(final_segments)} final segment(s), {len(result)} chars.")
    return result


async def live_session(
    stop_checker: Any = None,
    on_ready: Any = None,
) -> None:
    config = load_config()
    api_key = load_api_key()
    if websockets is None:
        raise RuntimeError("Python websockets module is missing. Install websockets.")

    try:
        RUNTIME_DIR.mkdir(parents=True, exist_ok=True)
        SESSION_LOG.write_text(
            f"=== AI Dikte Session: {time.strftime('%Y-%m-%d %H:%M:%S')} ===\n",
            encoding="utf-8",
        )
    except Exception:
        pass

    log_session_event(
        f"Session started. Platform: {'Windows' if IS_WINDOWS else 'Linux'}, "
        f"Model: {MODEL}, Language: {config_language(config)}"
    )

    def is_stop_requested() -> bool:
        if stop_checker is not None:
            return bool(stop_checker())
        return STOP_FILE.exists()

    ERROR_FILE.unlink(missing_ok=True)
    recorder_proc: asyncio.subprocess.Process | None = None
    sd_recorder: SoundDeviceStreamRecorder | None = None
    websocket = None
    connect_task: asyncio.Task[Any] | None = None
    receiver_task: asyncio.Task[None] | None = None
    transcripts: asyncio.Queue[tuple[str, str]] = asyncio.Queue()
    turn_complete_event = asyncio.Event()

    use_sounddevice = IS_WINDOWS

    try:
        loop = asyncio.get_running_loop()
        if use_sounddevice:
            input_device = config_input_device(config)
            sd_recorder = SoundDeviceStreamRecorder(device=input_device)
            sd_recorder.start(loop)
            log_session_event(
                f"SoundDevice recorder started. Device: {microphone_label(config)}, "
                f"Format: {SD_AUDIO_FORMAT}, Rate: {AUDIO_RATE}Hz"
            )
        else:
            # Keep the proven Linux PipeWire path explicit: raw signed 16-bit,
            # mono, 16 kHz PCM. Gemini receives exactly what the MIME type says.
            with ERROR_FILE.open("wb") as error_handle:
                recorder_proc = await asyncio.create_subprocess_exec(
                    "pw-record",
                    "--raw",
                    f"--rate={AUDIO_RATE}",
                    f"--channels={AUDIO_CHANNELS}",
                    f"--format={PW_AUDIO_FORMAT}",
                    "-",
                    stdin=asyncio.subprocess.DEVNULL,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=error_handle,
                    start_new_session=True,
                )
            await asyncio.sleep(0.18)
            if recorder_proc.returncode is not None:
                details = ERROR_FILE.read_text(errors="replace")[-500:] if ERROR_FILE.exists() else ""
                raise RuntimeError(f"Could not start recording. {details.strip()}")

        READY_FILE.write_text("ready\n", encoding="ascii")
        if not IS_WINDOWS:
            try:
                READY_FILE.chmod(0o600)
            except OSError:
                pass

        if on_ready is not None:
            try:
                on_ready()
            except Exception:
                pass

        notify("AI Dikte", "Recording started", event_type="start")
        connect_task = asyncio.create_task(open_live_websocket(api_key, config))
        buffered_audio: list[bytes] = []

        async def get_audio_chunk() -> bytes:
            if sd_recorder is not None:
                return await sd_recorder.read()
            assert recorder_proc is not None and recorder_proc.stdout is not None
            return await recorder_proc.stdout.read(AUDIO_CHUNK_BYTES)

        while not connect_task.done() and not is_stop_requested():
            try:
                chunk = await asyncio.wait_for(get_audio_chunk(), timeout=0.1)
            except asyncio.TimeoutError:
                continue
            if not chunk:
                details = ERROR_FILE.read_text(errors="replace")[-500:] if ERROR_FILE.exists() else ""
                raise RuntimeError(f"Recording stopped unexpectedly. {details.strip()}")
            buffered_audio.append(chunk)

        websocket = await connect_task
        receiver_task = asyncio.create_task(
            receive_transcriptions(websocket, transcripts, turn_complete_event)
        )
        await websocket.send(json.dumps({"realtimeInput": {"activityStart": {}}}))
        log_session_event(
            f"Sent activityStart. Flushing {len(buffered_audio)} buffered audio chunks."
        )

        has_sent_audio = False
        chunks_sent = 0

        for chunk in buffered_audio:
            await raise_if_receiver_failed(receiver_task)
            await send_audio(websocket, chunk)
            has_sent_audio = True
            chunks_sent += 1
        buffered_audio.clear()

        while not is_stop_requested():
            try:
                chunk = await asyncio.wait_for(get_audio_chunk(), timeout=0.1)
            except asyncio.TimeoutError:
                continue
            if not chunk:
                details = ERROR_FILE.read_text(errors="replace")[-500:] if ERROR_FILE.exists() else ""
                raise RuntimeError(f"Recording stopped unexpectedly. {details.strip()}")
            await raise_if_receiver_failed(receiver_task)
            await send_audio(websocket, chunk)
            has_sent_audio = True
            chunks_sent += 1

        log_session_event(
            f"Stop detected. Total chunks streamed during active recording: {chunks_sent}"
        )

        if sd_recorder is not None:
            # Stop capture first so the callback cannot race with the queue drain.
            sd_recorder.stop()
            drain_count = 0
            while not sd_recorder.queue.empty():
                try:
                    chunk = sd_recorder.queue.get_nowait()
                except asyncio.QueueEmpty:
                    break
                if chunk:
                    await raise_if_receiver_failed(receiver_task)
                    await send_audio(websocket, chunk)
                    has_sent_audio = True
                    chunks_sent += 1
                    drain_count += 1
            log_session_event(f"Drained {drain_count} pending SoundDevice audio chunks.")
        elif recorder_proc is not None:
            await stop_recorder(recorder_proc)

        # Drain remaining PipeWire stdout after the recorder has stopped.
        if recorder_proc is not None and recorder_proc.stdout is not None:
            while True:
                chunk = await recorder_proc.stdout.read(AUDIO_CHUNK_BYTES)
                if not chunk:
                    break
                await raise_if_receiver_failed(receiver_task)
                await send_audio(websocket, chunk)
                has_sent_audio = True
                chunks_sent += 1

        if not has_sent_audio:
            dummy_chunk = b"\x00\x00" * 1600
            await send_audio(websocket, dummy_chunk)
            log_session_event("Sent one silence chunk for empty-recording guard.")

        await websocket.send(json.dumps({"realtimeInput": {"activityEnd": {}}}))
        log_session_event(
            f"Sent activityEnd. Total chunks sent: {chunks_sent}. Collecting transcript."
        )

        text = await collect_final_transcript(
            transcripts,
            receiver_task,
            turn_complete_event,
        )
        log_session_event(f"Injecting transcription ({len(text)} chars).")
        driver = output_text(text)
        log_session_event(f"Text injected successfully via {driver}.")
        play_audio_cue("finish")
        notify("AI Dikte", f"Text ready ({driver})", event_type="finish")
    except Exception as exc:
        log_session_event(
            f"EXCEPTION in live_session: {type(exc).__name__}: {exc}"
        )
        raise
    finally:
        if sd_recorder is not None:
            try:
                sd_recorder.stop()
            except Exception:
                pass
        if recorder_proc is not None:
            try:
                await stop_recorder(recorder_proc)
            except Exception:
                pass
        if connect_task is not None and not connect_task.done():
            connect_task.cancel()
            try:
                await connect_task
            except (asyncio.CancelledError, Exception):
                pass
        if receiver_task is not None and not receiver_task.done():
            receiver_task.cancel()
            try:
                await receiver_task
            except (asyncio.CancelledError, Exception):
                pass
        if websocket is not None:
            try:
                await websocket.close()
            except Exception:
                pass
        clear_runtime_state()
        log_session_event("Session ended and cleaned up.")


def hide_private_windows_console() -> None:
    """Hide a daemon-only console without hiding the caller's terminal."""
    if not IS_WINDOWS:
        return
    try:
        process_ids = (wintypes.DWORD * 2)()
        attached_processes = ctypes.windll.kernel32.GetConsoleProcessList(
            process_ids,
            len(process_ids),
        )
        if attached_processes != 1:
            return
        hwnd = ctypes.windll.kernel32.GetConsoleWindow()
        if hwnd:
            ctypes.windll.user32.ShowWindow(hwnd, 0)
    except Exception:
        pass


def independent_frozen_env() -> dict[str, str] | None:
    """Start a new one-file app instance with its own extraction directory."""
    if not getattr(sys, "frozen", False):
        return None
    child_env = os.environ.copy()
    child_env["PYINSTALLER_RESET_ENVIRONMENT"] = "1"
    return child_env


def application_command(*arguments: str) -> list[str]:
    if getattr(sys, "frozen", False):
        return [sys.executable, *arguments]
    return [sys.executable, str(Path(__file__).with_name("ai_dikte.py")), *arguments]


def windows_daemon_command() -> list[str]:
    return application_command("daemon")


REG_RUN_PATH = r"Software\Microsoft\Windows\CurrentVersion\Run"
REG_RUN_NAME = "AI-Dikte"


def windows_startup_enabled() -> bool:
    if not IS_WINDOWS:
        return False
    try:
        import winreg

        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, REG_RUN_PATH, 0, winreg.KEY_READ) as key:
            val, _ = winreg.QueryValueEx(key, REG_RUN_NAME)
            return bool(val)
    except FileNotFoundError:
        return False


def set_windows_startup_enabled(enabled: bool) -> None:
    if not IS_WINDOWS:
        raise RuntimeError("Startup management is only available on Windows.")
    import winreg

    with winreg.CreateKey(winreg.HKEY_CURRENT_USER, REG_RUN_PATH) as key:
        if enabled:
            command = subprocess.list2cmdline(windows_daemon_command())
            winreg.SetValueEx(key, REG_RUN_NAME, 0, winreg.REG_SZ, command)
        else:
            try:
                winreg.DeleteValue(key, REG_RUN_NAME)
            except FileNotFoundError:
                pass

def launch_windows_daemon_restart() -> None:
    if not IS_WINDOWS:
        raise RuntimeError("Daemon restart is only available on Windows.")
    command = application_command("_restart-daemon", str(os.getpid()))
    subprocess.Popen(
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        creationflags=0x08000000 | 0x00000008,
        env=independent_frozen_env(),
        close_fds=True,
    )


def restart_windows_daemon(parent_pid: int) -> None:
    deadline = time.monotonic() + 8.0
    while pid_alive(parent_pid) and time.monotonic() < deadline:
        time.sleep(0.1)
    subprocess.Popen(
        windows_daemon_command(),
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        creationflags=0x08000000 | 0x00000008,
        env=independent_frozen_env(),
        close_fds=True,
    )


def open_session_log() -> None:
    RUNTIME_DIR.mkdir(parents=True, exist_ok=True)
    SESSION_LOG.touch(exist_ok=True)
    if IS_WINDOWS:
        os.startfile(SESSION_LOG)
    else:
        print(SESSION_LOG)


def doctor_report() -> tuple[str, bool]:
    import contextlib
    import io

    output = io.StringIO()
    failed = False
    with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
        try:
            doctor()
        except SystemExit as exc:
            failed = bool(exc.code)
    return output.getvalue().strip() or "No diagnostic output was produced.", not failed


def copy_windows_clipboard(text: str) -> None:
    """Copy Unicode text through the Win32 clipboard API."""
    if not IS_WINDOWS:
        raise RuntimeError("Clipboard diagnostics are only available on Windows.")

    user32 = ctypes.windll.user32
    kernel32 = ctypes.windll.kernel32
    cf_unicode_text = 13
    gmem_moveable = 0x0002
    payload = text.encode("utf-16-le") + b"\x00\x00"

    user32.OpenClipboard.argtypes = [wintypes.HWND]
    user32.OpenClipboard.restype = wintypes.BOOL
    user32.SetClipboardData.argtypes = [wintypes.UINT, wintypes.HANDLE]
    user32.SetClipboardData.restype = wintypes.HANDLE
    kernel32.GlobalAlloc.argtypes = [wintypes.UINT, ctypes.c_size_t]
    kernel32.GlobalAlloc.restype = wintypes.HGLOBAL
    kernel32.GlobalLock.argtypes = [wintypes.HGLOBAL]
    kernel32.GlobalLock.restype = ctypes.c_void_p
    kernel32.GlobalUnlock.argtypes = [wintypes.HGLOBAL]
    kernel32.GlobalFree.argtypes = [wintypes.HGLOBAL]

    for _attempt in range(10):
        if user32.OpenClipboard(None):
            break
        time.sleep(0.025)
    else:
        raise RuntimeError("The Windows clipboard is currently busy.")

    memory_handle = None
    clipboard_owns_memory = False
    try:
        memory_handle = kernel32.GlobalAlloc(gmem_moveable, len(payload))
        if not memory_handle:
            raise ctypes.WinError()
        memory_pointer = kernel32.GlobalLock(memory_handle)
        if not memory_pointer:
            raise ctypes.WinError()
        try:
            ctypes.memmove(memory_pointer, payload, len(payload))
        finally:
            kernel32.GlobalUnlock(memory_handle)

        if not user32.EmptyClipboard():
            raise ctypes.WinError()
        if not user32.SetClipboardData(cf_unicode_text, memory_handle):
            raise ctypes.WinError()
        clipboard_owns_memory = True
    finally:
        user32.CloseClipboard()
        if memory_handle and not clipboard_owns_memory:
            kernel32.GlobalFree(memory_handle)


def copy_doctor_report() -> None:
    report, _ = doctor_report()
    copy_windows_clipboard(report)


def tray_child_command(subcmd: str) -> list[str]:
    return application_command("_tray-" + subcmd)


def launch_tray_command(subcmd: str) -> subprocess.Popen:
    """Launch setup/doctor without exposing the daemon's hidden console."""
    creationflags = 0x08000000 if IS_WINDOWS else 0  # CREATE_NO_WINDOW
    return subprocess.Popen(
        tray_child_command(subcmd),
        creationflags=creationflags,
        env=independent_frozen_env(),
    )


def run_tray_gui_command(subcmd: str) -> bool:
    from ai_dikte_ui import SettingsServices, show_dialog
    services = SettingsServices(
        load_config=lambda: build_setup_config(load_config(required=False)),
        load_key=lambda: load_api_key(required=False),
        save=save_settings,
        diagnostics=doctor_report,
        lock=lambda: FileLock(LOCK_FILE),
        devices=list_input_devices if IS_WINDOWS else None,
        get_startup=windows_startup_enabled if IS_WINDOWS else None,
        set_startup=set_windows_startup_enabled if IS_WINDOWS else None,
    )
    return show_dialog(subcmd, services)


def run_daemon() -> None:
    """Run in background with global hotkey and system tray icon."""
    global _GLOBAL_TRAY_ICON
    hide_private_windows_console()


    if not IS_WINDOWS:
        raise RuntimeError("Linux uses the desktop Meta+Z shortcut; daemon mode is Windows-only.")
    config = load_config()
    hotkey = config_hotkey(config)

    print("[*] Starting AI Dikte daemon...")
    print(f"[*] Dictation hotkey: {hotkey}")

    if not IS_WINDOWS:
        raise RuntimeError("Linux uses the desktop Meta+Z shortcut; daemon mode is Windows-only.")
    get_windows_osd()
    setup_windows_keyboard_hook(toggle, log_session_event)
    print("[OK] Global Windows hook registered for: Win+Z")

    tray_children: dict[str, subprocess.Popen] = {}

    try:
        from PIL import Image, ImageDraw
        import pystray

        def render_tray_icon(bg_color: str, symbol: str = "mic") -> Image.Image:
            s = 128
            img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
            draw = ImageDraw.Draw(img)
            margin = 4
            draw.ellipse([(margin, margin), (s - margin, s - margin)], fill=bg_color)

            cx, cy = s // 2, s // 2 - 4
            cw, ch = 24, 44
            cr = cw // 2
            cbox = [(cx - cr, cy - ch // 2), (cx + cr, cy + ch // 2)]
            draw.rounded_rectangle(cbox, radius=cr, fill=(255, 255, 255, 255))

            arc_r = cr + 9
            cradle_top = cy - 6
            draw.arc(
                [(cx - arc_r, cradle_top - arc_r), (cx + arc_r, cradle_top + arc_r)],
                start=0,
                end=180,
                fill=(255, 255, 255, 255),
                width=5,
            )

            stem_top = cradle_top + arc_r
            stem_bot = stem_top + 16
            draw.line([(cx, stem_top), (cx, stem_bot)], fill=(255, 255, 255, 255), width=5)
            draw.line([(cx - 18, stem_bot), (cx + 18, stem_bot)], fill=(255, 255, 255, 255), width=5)

            if symbol == "sparkle":
                sx, sy, sz = cx + 26, cy - 20, 10
                pts = [
                    (sx, sy - sz),
                    (sx + sz * 0.3, sy - sz * 0.3),
                    (sx + sz, sy),
                    (sx + sz * 0.3, sy + sz * 0.3),
                    (sx, sy + sz),
                    (sx - sz * 0.3, sy + sz * 0.3),
                    (sx - sz, sy),
                    (sx - sz * 0.3, sy - sz * 0.3),
                ]
                draw.polygon(pts, fill=(255, 255, 255, 255))
            elif symbol == "recording":
                rx, ry = cx + 26, cy - 20
                draw.ellipse([(rx - 7, ry - 7), (rx + 7, ry + 7)], fill=(255, 255, 255, 255))

            return img.resize((64, 64), Image.Resampling.LANCZOS)

        images = {
            "blue": render_tray_icon("#4f46e5", "mic"),
            "red": render_tray_icon("#ef4444", "recording"),
            "yellow": render_tray_icon("#eab308", "sparkle"),
            "gray": render_tray_icon("#6b7280", "mic"),
        }

        def on_toggle_clicked(icon, item):
            toggle()

        def on_exit_clicked(icon, item):
            icon.stop()

        def on_open_logs_clicked(icon, item):
            try:
                open_session_log()
            except OSError as exc:
                notify("AI Dikte", str(exc), "critical", event_type="error")

        def on_copy_diagnostics_clicked(icon, item):
            try:
                copy_doctor_report()
                msg = "Diagnostics copied to clipboard."
                notify("AI Dikte", msg)
            except Exception as exc:
                notify("AI Dikte", str(exc), "critical", event_type="error")

        def on_startup_clicked(icon, item):
            try:
                new_state = not windows_startup_enabled()
                set_windows_startup_enabled(new_state)
                icon.update_menu()
                msg = f"Start with Windows {'enabled' if new_state else 'disabled'}."
                notify("AI Dikte", msg)
            except OSError as exc:
                notify("AI Dikte", str(exc), "critical", event_type="error")

        def on_restart_clicked(icon, item):
            try:
                launch_windows_daemon_restart()
                icon.stop()
            except OSError as exc:
                notify("AI Dikte", str(exc), "critical", event_type="error")

        def run_child_cmd(subcmd: str):
            existing = tray_children.get(subcmd)
            if existing is not None and existing.poll() is None:
                msg = f"{subcmd.title()} is already open."
                notify("AI Dikte", msg)
                return

            try:
                tray_children[subcmd] = launch_tray_command(subcmd)
            except OSError as exc:
                notify(
                    "AI Dikte",
                    f"Could not open {subcmd}: {exc}",
                    "critical",
                    event_type="error",
                )

        menu_items = [
            pystray.MenuItem(f"Toggle Dictation ({hotkey})", on_toggle_clicked, default=True),
            pystray.MenuItem('Settings', lambda: run_child_cmd("setup")),
            pystray.MenuItem('Doctor', lambda: run_child_cmd("doctor")),
        ]
        if IS_WINDOWS:
            menu_items.extend(
                [
                    pystray.Menu.SEPARATOR,
                    pystray.MenuItem(
                        lambda _item: 'Microphone: {mic}'.format(mic=microphone_label(load_config(required=False))),
                        None,
                        enabled=False,
                    ),
                    pystray.MenuItem(
                        lambda _item: 'Model: {model}'.format(model=MODEL),
                        None,
                        enabled=False,
                    ),
                    pystray.MenuItem('Open Logs', on_open_logs_clicked),
                    pystray.MenuItem('Copy Diagnostics', on_copy_diagnostics_clicked),
                    pystray.MenuItem(
                        'Start with Windows',
                        on_startup_clicked,
                        checked=lambda _item: windows_startup_enabled(),
                    ),
                    pystray.MenuItem('Restart', on_restart_clicked),
                ]
            )
        menu_items.extend([
            pystray.Menu.SEPARATOR,
            pystray.MenuItem('Exit', on_exit_clicked),
        ])
        menu = pystray.Menu(*menu_items)

        tray_icon = pystray.Icon(
            "ai-dikte",
            images["blue"],
            f"AI Dikte — Ready ({hotkey})",
            menu,
        )
        _GLOBAL_TRAY_ICON = tray_icon

        def tray_state_monitor():
            current_state = None
            last_startup = windows_startup_enabled() if IS_WINDOWS else None
            last_config_mtime = CONFIG_FILE.stat().st_mtime if CONFIG_FILE.exists() else None
            while tray_icon and tray_icon.visible:
                time.sleep(0.3)
                if IS_WINDOWS:
                    current_startup = windows_startup_enabled()
                    current_mtime = CONFIG_FILE.stat().st_mtime if CONFIG_FILE.exists() else None
                    if current_startup != last_startup or current_mtime != last_config_mtime:
                        last_startup = current_startup
                        last_config_mtime = current_mtime
                        try:
                            tray_icon.update_menu()
                        except Exception:
                            pass

                if IS_WINDOWS and _WINDOWS_SESSION_CONTROLLER is not None:
                    st = _WINDOWS_SESSION_CONTROLLER.get_state()
                    if st == "recording":
                        new_state = ("red", 'AI Dikte — Listening...')
                    elif st == "transcribing":
                        new_state = ("yellow", 'AI Dikte — Transcribing...')
                    elif st == "starting":
                        new_state = ("yellow", 'AI Dikte — Starting...')
                    else:
                        new_state = ("blue", f"AI Dikte — Ready ({hotkey})")
                else:
                    pid = read_pid()
                    is_active = bool(pid and session_matches(pid))
                    if is_active:
                        if STOP_FILE.exists():
                            new_state = ("yellow", 'AI Dikte — Transcribing...')
                        elif READY_FILE.exists():
                            new_state = ("red", 'AI Dikte — Listening...')
                        else:
                            new_state = ("yellow", 'AI Dikte — Starting...')
                    else:
                        new_state = ("blue", f"AI Dikte — Ready ({hotkey})")

                if new_state != current_state and tray_icon:
                    current_state = new_state
                    try:
                        tray_icon.icon = images.get(new_state[0], images["blue"])
                        tray_icon.title = new_state[1]
                    except Exception:
                        pass
        def tray_setup(icon):
            icon.visible = True
            threading.Thread(target=tray_state_monitor, daemon=True).start()

        print("[OK] System Tray icon active.")
        tray_icon.run(setup=tray_setup)
    except ImportError as exc:
        raise RuntimeError(f"Required Windows tray dependency is missing: {exc}") from exc


def doctor() -> None:
    config = load_config(required=False)
    driver = available_output_driver(config)
    desktop = desktop_kind()

    if IS_WINDOWS:
        checks = {
            "platform-windows": True,
            "sounddevice": sounddevice is not None,
            "microphone": microphone_available(config),
            "python-websockets": websockets is not None,
            "direct-typing (SendInput)": bool(driver),
            "config": CONFIG_FILE.exists(),
            "api-key (Credential Manager)": bool(load_api_key(required=False)),
        }
    else:
        checks = {
            "wayland": bool(os.environ.get("WAYLAND_DISPLAY")),
            "pw-record": bool(shutil.which("pw-record")),
            "notify-send": bool(shutil.which("notify-send")),
            "python-websockets": websockets is not None,
            "config": CONFIG_FILE.exists(),
            "api-key": bool(str(config.get("api_key", "")).strip()),
            "direct-typing": bool(driver),
        }

    for name, ok in checks.items():
        if name in ("direct-typing", "direct-typing (SendInput)") and driver:
            suffix = f" ({driver})"
        elif name == "microphone":
            suffix = f" ({microphone_label(config)})"
        else:
            suffix = ""
        status_mark = "[OK]" if ok else "[FAIL]"
        print(f"{status_mark} {name}{suffix}")

    print(f"i desktop/os: {desktop}")
    print(f"i model: {MODEL}")
    if IS_WINDOWS:
        print("i hotkey: win+z")

    if not IS_WINDOWS and desktop == "hyprland":
        target, _, start_marker, _ = hyprland_shortcut_target()
        installed = False
        if target.exists():
            try:
                content = target.read_text(encoding="utf-8")
                installed = (
                    start_marker in content
                    or HYPR_MARKER_START_CONF in content
                    or HYPR_MARKER_START_LUA in content
                )
            except OSError:
                pass
        print(
            f"{'[OK]' if installed else '[FAIL]'} "
            f"Meta+Z Hyprland binding ({target.name})"
        )

    if not all(checks.values()):
        raise SystemExit(1)


def main() -> None:
    ensure_runtime()
    command = sys.argv[1] if len(sys.argv) > 1 else "toggle"

    if command == "_live-session":
        try:
            asyncio.run(live_session())
        except Exception as exc:
            notify("AI Dikte — Error", str(exc), "critical", event_type="error")
            print(f"[ERROR] {exc}", file=sys.stderr)
            clear_runtime_state()
            raise SystemExit(1)
        return
    if command == "_restart-daemon":
        if not IS_WINDOWS or len(sys.argv) != 3:
            raise RuntimeError("Invalid daemon restart command.")
        restart_windows_daemon(int(sys.argv[2]))
        return


    if command.startswith("_tray-"):
        run_tray_gui_command(command.removeprefix("_tray-"))
        return

    if command in ("daemon", "run"):
        run_daemon()
        return

    if command == "setup":
        setup()
        return

    with FileLock(LOCK_FILE):
        if command == "toggle":
            toggle()
        elif command == "setup":
            setup()
        elif command == "doctor":
            doctor()
        elif command == "shortcut-install":
            install_hyprland_shortcut()
        elif command == "shortcut-remove":
            remove_hyprland_shortcut()
        else:
            print(
                "Usage: ai-dikte "
                "{toggle|daemon|setup|doctor|shortcut-install|shortcut-remove}"
            )
            raise SystemExit(2)

