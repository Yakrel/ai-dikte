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

# Install all build & runtime dependencies in a single transaction
echo -e "${BOLD}${BLUE}==>${NC} Installing required dependencies..."
sudo pacman -S --needed --noconfirm \
    base-devel tar meson ninja pkgconf \
    kwayland libnotify libxkbcommon pipewire-audio python python-websockets \
    qt6-base wayland wtype

echo -e "${BOLD}${BLUE}==>${NC} Downloading latest source from GitHub (git-free)..."
curl -fsSL "https://github.com/Yakrel/ai-dikte/archive/refs/heads/main.tar.gz" | tar -xz -C "$TEMP_DIR" --strip-components=1

echo -e "${BOLD}${BLUE}==>${NC} Building and installing package with makepkg..."
cd "$TEMP_DIR"
makepkg -si --noconfirm --needed

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
