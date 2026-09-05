"""Configuration validation and persistence; no UI or recording side effects."""
from __future__ import annotations
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import Any

IS_WINDOWS = sys.platform == "win32"
APP = "ai-dikte"
DEFAULT_LANGUAGE = "tr-TR"
DEFAULT_MODE = "SMART"
DEFAULT_HOTKEY = "win+z" if IS_WINDOWS else "SUPER + Z"
CONFIG_DIR = Path(os.environ.get("APPDATA" if IS_WINDOWS else "XDG_CONFIG_HOME", Path.home() / ".config")) / APP
CONFIG_FILE = CONFIG_DIR / "config.json"
if IS_WINDOWS:
    from ai_dikte_win32 import read_windows_api_key, write_windows_api_key

def load_config(required: bool = True) -> dict[str, Any]:
    if not CONFIG_FILE.exists():
        if required:
            raise RuntimeError("Configuration missing. Run 'ai-dikte setup'.")
        return {}

    try:
        config = json.loads(CONFIG_FILE.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise RuntimeError(f"Could not read config: {exc}")

    if not isinstance(config, dict):
        raise RuntimeError("Configuration is not a JSON object.")
    return config


def write_config(config: dict[str, Any]) -> None:
    """Atomically replace config.json so interrupted writes cannot corrupt it."""
    CONFIG_FILE.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(
        prefix=f".{CONFIG_FILE.name}.",
        suffix=".tmp",
        dir=CONFIG_FILE.parent,
    )
    temp_path = Path(temp_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as handle:
            json.dump(config, handle, indent=2, ensure_ascii=False)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        if not IS_WINDOWS:
            temp_path.chmod(0o600)
        os.replace(temp_path, CONFIG_FILE)
    finally:
        temp_path.unlink(missing_ok=True)


def load_api_key(required: bool = True) -> str:
    if IS_WINDOWS:
        key = read_windows_api_key()
    else:
        key = str(load_config(required=False).get("api_key", "")).strip()

    if required and not key:
        raise RuntimeError("API key missing. Run 'ai-dikte setup'.")
    return key


def config_language(config: dict[str, Any]) -> str:
    value = str(config.get("language", DEFAULT_LANGUAGE)).strip()
    return value or DEFAULT_LANGUAGE


def config_mode(config: dict[str, Any]) -> str:
    value = str(config.get("mode", DEFAULT_MODE)).strip().upper()
    if value not in {"SMART", "VERBATIM"}:
        raise RuntimeError("Config 'mode' must be SMART or VERBATIM.")
    return value


def config_vocabulary(config: dict[str, Any]) -> list[str]:
    raw = config.get("custom_vocabulary", [])
    if raw is None:
        return []
    if not isinstance(raw, list):
        raise RuntimeError("Config 'custom_vocabulary' must be a JSON array.")

    result: list[str] = []
    for item in raw:
        term = str(item).strip()
        if term and term not in result:
            result.append(term)
    if len(result) > 1000:
        raise RuntimeError("Config 'custom_vocabulary' supports at most 1000 terms.")
    return result


def config_output_driver(config: dict[str, Any]) -> str:
    driver = str(config.get("output_driver", "auto")).strip().lower() or "auto"
    if IS_WINDOWS:
        if driver not in {"auto", "sendinput"}:
            raise RuntimeError("Config 'output_driver' on Windows must be auto or sendinput.")
        return driver
    if driver not in {"auto", "kwtype", "wtype"}:
        raise RuntimeError("Config 'output_driver' on Linux must be auto, kwtype, or wtype.")
    return driver


def config_hotkey(config: dict[str, Any]) -> str:
    # Win+Z is intentionally fixed on Windows because it must override the
    # Windows 11 Snap Layout shortcut using the low-level hook.
    if IS_WINDOWS:
        return "win+z"
    value = str(config.get("hotkey", DEFAULT_HOTKEY)).strip()
    return value or DEFAULT_HOTKEY


def config_audio_cue(config: dict[str, Any]) -> bool:
    value = config.get("audio_cue", True)
    if not isinstance(value, bool):
        raise RuntimeError("Config 'audio_cue' must be true or false.")
    return value


def config_notify_mode(config: dict[str, Any]) -> str:
    value = str(config.get("notify_mode", "all")).strip().lower() or "all"
    if value not in {"all", "none"}:
        raise RuntimeError("Config 'notify_mode' must be all or none.")
    return value


def config_input_device(config: dict[str, Any]) -> int | None:
    value = config.get("input_device")
    if value is None or value == "" or str(value).lower() == "default":
        return None
    if isinstance(value, bool):
        raise RuntimeError("Config 'input_device' must be a device index or null.")
    try:
        index = int(value)
    except (TypeError, ValueError) as exc:
        raise RuntimeError("Config 'input_device' must be a device index or null.") from exc
    if index < 0:
        raise RuntimeError("Config 'input_device' cannot be negative.")
    return index


def build_setup_config(
    existing: dict[str, Any],
    updates: dict[str, Any] | None = None,
) -> dict[str, Any]:
    config = dict(existing)
    if updates:
        config.update(updates)
    config.pop("ui_language", None)
    config.update(
        {
            "language": config_language(config),
            "mode": config_mode(config),
            "custom_vocabulary": config_vocabulary(config),
            "output_driver": config_output_driver(config),
            "hotkey": config_hotkey(config),
            "audio_cue": config_audio_cue(config),
            "notify_mode": config_notify_mode(config),
            "input_device": config_input_device(config),

        }
    )
    return config


def save_setup_config(key: str, config: dict[str, Any] | None = None) -> None:
    key = key.strip()
    if not key:
        raise RuntimeError("API key cannot be empty.")
    config = build_setup_config(
        load_config(required=False) if config is None else config
    )

    if IS_WINDOWS:
        write_windows_api_key(key)
        config.pop("api_key", None)
    else:
        config["api_key"] = key
    write_config(config)
    print(f"[OK] Saved: {CONFIG_FILE}")



