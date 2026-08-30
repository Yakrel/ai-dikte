#!/usr/bin/env bash
set -euo pipefail

# AI Dikte One-Line Installer for Arch Linux, CachyOS & Omarchy (Git-Free)
# Usage: bash -c "$(curl -fsSL https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.sh)"

BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BOLD}${BLUE}==>${NC} ${BOLD}AI Dikte Installer (Arch Linux / CachyOS / Omarchy)${NC}"

# Prevent running directly as root/sudo
if [ "${EUID:-$(id -u)}" -eq 0 ]; then
    echo -e "${RED}[ERROR]${NC} Please run this script as a normal user (do not use sudo bash ...)."
    echo -e "makepkg requires standard user privileges and will request sudo when installing packages."
    echo -e "Run simply as: ${BOLD}bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.sh)\"${NC}"
    exit 1
fi

# Check for pacman
if ! command -v pacman >/dev/null 2>&1; then
    echo -e "${RED}[ERROR]${NC} pacman not found. This installer requires Arch Linux, CachyOS, Omarchy, or an Arch-based system."
    exit 1
fi

# Detect the active Wayland desktop so only its required typing backend is installed.
desktop_env="$(printf '%s %s %s' \
    "${XDG_CURRENT_DESKTOP:-}" \
    "${XDG_SESSION_DESKTOP:-}" \
    "${DESKTOP_SESSION:-}" | tr '[:upper:]' '[:lower:]')"

if [ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ] || [[ "$desktop_env" == *hyprland* ]]; then
    DESKTOP_KIND="hyprland"
    TYPING_BACKEND="wtype"
elif [ -n "${KDE_FULL_SESSION:-}" ] || [[ "$desktop_env" == *kde* ]] || [[ "$desktop_env" == *plasma* ]]; then
    DESKTOP_KIND="kde"
    TYPING_BACKEND="kwtype"
else
    echo -e "${RED}[ERROR]${NC} Unsupported desktop session."
    echo "AI Dikte currently supports Hyprland/Omarchy and KDE Plasma on Wayland."
    echo "Detected desktop environment: ${desktop_env:-unknown}"
    exit 1
fi

echo -e "${BOLD}${BLUE}==>${NC} Detected ${BOLD}${DESKTOP_KIND}${NC}; using only ${BOLD}${TYPING_BACKEND}${NC} for direct typing."

# Authenticate sudo once upfront and maintain active session
echo -e "${BOLD}${BLUE}==>${NC} Requesting sudo permissions (one-time prompt)..."
sudo -v

# Keep sudo session alive in background until script exits
while true; do sudo -n true; sleep 45; kill -0 "$$" || exit; done 2>/dev/null &
SUDO_KEEPALIVE_PID=$!

TEMP_DIR=$(mktemp -d "/tmp/ai-dikte-installer-XXXXXX")
cleanup() {
    kill "$SUDO_KEEPALIVE_PID" 2>/dev/null || true
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

# makepkg requires the standard Arch build environment. Omarchy already ships
# base-devel, so --needed makes this a no-op there.
echo -e "${BOLD}${BLUE}==>${NC} Ensuring the minimal build environment is available..."
sudo pacman -S --needed --noconfirm base-devel tar

# Hyprland/Omarchy needs only wtype. KDE gets a dedicated KWtype package below.
if [ "$DESKTOP_KIND" = "hyprland" ]; then
    echo -e "${BOLD}${BLUE}==>${NC} Installing Hyprland typing backend..."
    sudo pacman -S --needed --noconfirm wtype
fi

echo -e "${BOLD}${BLUE}==>${NC} Downloading latest source from GitHub (git-free)..."
curl -fsSL "https://github.com/Yakrel/ai-dikte/archive/refs/heads/main.tar.gz" | tar -xz -C "$TEMP_DIR" --strip-components=1

cd "$TEMP_DIR"

if [ "$DESKTOP_KIND" = "kde" ]; then
    if command -v kwtype >/dev/null 2>&1; then
        echo -e "${BOLD}${BLUE}==>${NC} Existing KWtype detected; no KDE backend build needed."
    else
        echo -e "${BOLD}${BLUE}==>${NC} Building KDE typing backend (KWtype)..."
        # --rmdeps removes build-only packages pulled in by this helper build.
        # KWtype itself stays explicitly installed so orphan cleanup cannot remove
        # the active text-injection backend behind AI Dikte's back.
        makepkg -p PKGBUILD.kwtype --syncdeps --rmdeps --install --clean \
            --noconfirm --needed
    fi
fi

echo -e "${BOLD}${BLUE}==>${NC} Building and installing AI Dikte..."
makepkg --syncdeps --install --clean --noconfirm --needed

echo -e "${GREEN}==>${NC} ${BOLD}AI Dikte installed successfully!${NC}"
echo ""

# Run interactive setup
if [ -t 0 ]; then
    echo -e "${BOLD}${BLUE}==>${NC} Running initial configuration..."
    ai-dikte setup
    echo ""
    echo -e "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
    ai-dikte doctor || true
else
    echo -e "${BOLD}To configure your Gemini API key, run:${NC}"
    echo "  ai-dikte setup"
    echo "  ai-dikte doctor"
fi

echo ""
echo -e "${GREEN}${BOLD}Setup complete!${NC} Press ${BOLD}Meta+Z${NC} to start dictation."
