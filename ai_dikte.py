#!/usr/bin/env python3
"""Windows & cross-platform entrypoint for AI Dikte."""

from __future__ import annotations

import runpy
import sys
from pathlib import Path

if __name__ == "__main__":
    script_path = Path(__file__).resolve().parent / "ai-dikte"
    if not script_path.exists():
        # Fallback if bundled or renamed
        script_path = Path(__file__).resolve().with_name("ai-dikte")
    runpy.run_path(str(script_path), run_name="__main__")
