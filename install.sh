#!/usr/bin/env bash
set -euo pipefail

SOURCE_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
TARGET="$BIN_DIR/gemini-dikte"
WRAPPER="$BIN_DIR/gemini-dikte-toggle.sh"

if command -v pacman >/dev/null 2>&1; then
    echo "==> Checking required CachyOS/Arch packages..."
    sudo pacman -S --needed pipewire-audio wl-clipboard ydotool libnotify
fi

mkdir -p "$BIN_DIR"
install -m 0755 "$SOURCE_DIR/gemini-dikte" "$TARGET"
echo "[OK] Installed: $TARGET"

# In KDE Plasma 6, extensionless scripts in Command or Script shortcut
# may fail silently on some systems.
# We create a small .sh wrapper for the shortcut.
cat > "$WRAPPER" <<EOF_WRAPPER
#!/bin/sh
exec "$TARGET" toggle
EOF_WRAPPER
chmod 0755 "$WRAPPER"
echo "[OK] KDE shortcut wrapper created: $WRAPPER"

missing=()
for command in pw-record wl-copy ydotool notify-send; do
    command -v "$command" >/dev/null 2>&1 || missing+=("$command")
done
if ((${#missing[@]})); then
    echo "[WARNING] Missing commands: ${missing[*]}"
fi

# Uses ydotool's udev rule and user service.
if command -v ydotoold >/dev/null 2>&1; then
    sudo modprobe uinput 2>/dev/null || true
    if ! id -nG "$USER" | tr ' ' '\n' | grep -qx input; then
        sudo usermod -aG input "$USER"
        echo "[WARNING] Added to 'input' group. Log out and log back in for auto-paste to work."
    fi
    systemctl --user enable --now ydotool.service 2>/dev/null || \
        echo "[WARNING] ydotool service could not be started; text will still be copied to clipboard."
fi

if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "[WARNING] $BIN_DIR is not in PATH. You can use full path in terminal:"
    echo "  $TARGET"
fi

echo
echo "Now configure your API key:"
"$TARGET" setup

echo
echo "============================================================"
echo "KDE Plasma Shortcut"
echo "============================================================"
echo "System Settings -> Keyboard -> Shortcuts -> Add New -> Command or Script"
echo "Command:"
echo "  $WRAPPER"
echo "Recommended shortcut: Meta+Z"
echo
echo "Note: Use the .sh wrapper above instead of '$TARGET toggle' directly in KDE shortcut."
