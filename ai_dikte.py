#!/usr/bin/env python3
"""Windows & cross-platform entrypoint for AI Dikte."""

from __future__ import annotations

import runpy
import sys
from pathlib import Path

VERSION = "0.1.0"


def bundled_script_path() -> Path:
    """Return the real ai-dikte script in source and PyInstaller builds."""
    if getattr(sys, "frozen", False):
        bundle_dir = Path(getattr(sys, "_MEIPASS", Path(sys.executable).resolve().parent))
        return bundle_dir / "ai-dikte"
    return Path(__file__).resolve().parent / "ai-dikte"


def normalize_frozen_child_argv() -> None:
    """Fix child commands spawned by the source script when running frozen.

    The shared ai-dikte source launches child Python commands as
    [sys.executable, __file__, command]. In a PyInstaller build sys.executable is
    the .exe, so the bundled script path arrives as argv[1]. Strip that path so
    the frozen entrypoint sees the intended command.
    """
    if not getattr(sys, "frozen", False) or len(sys.argv) < 3:
        return

    candidate = Path(sys.argv[1])
    if candidate.name in {"ai-dikte", "ai_dikte.py"}:
        sys.argv = [sys.argv[0], *sys.argv[2:]]


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
        import keyboard

        if not callable(getattr(keyboard, "add_hotkey", None)):
            failures.append("keyboard.add_hotkey unavailable")
    except Exception as exc:  # pragma: no cover - exercised in frozen CI
        failures.append(f"keyboard: {exc}")

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

    runpy.run_path(str(script_path), run_name="__main__")


if __name__ == "__main__":
    main()
