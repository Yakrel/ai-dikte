#!/usr/bin/env bash
set -euo pipefail

rm -f "$HOME/.local/bin/gemini-dikte"
rm -f "$HOME/.local/bin/gemini-dikte-toggle.sh"
rm -rf "$HOME/.config/gemini-dikte"
rm -rf "$HOME/.local/state/gemini-dikte"
rm -f "$HOME/.local/state/gemini-dikte.log"
rm -rf "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/gemini-dikte"

echo "[OK] Gemini Dictation uninstalled."
echo "Remember to manually remove any custom shortcut from KDE System Settings."
