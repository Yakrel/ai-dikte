#!/usr/bin/env python3
"""Windows & cross-platform entrypoint for AI Dikte."""

from __future__ import annotations

import runpy
import sys
from pathlib import Path

VERSION = "0.1.0"

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] in {"--version", "version"}:
        print(f"AI Dikte {VERSION}")
        raise SystemExit(0)

    script_path = Path(__file__).resolve().parent / "ai-dikte"
    if not script_path.exists():
        # PyInstaller extracts bundled data next to the frozen entrypoint.
        script_path = Path(__file__).resolve().with_name("ai-dikte")
    runpy.run_path(str(script_path), run_name="__main__")
