# AI Dikte

Cross-platform, minimal AI-powered dictation for **Linux** (Arch Linux, CachyOS, KDE Plasma, Hyprland, Omarchy) and **Windows** (10 / 11) using **Gemini 3.5 Transcribe Live**.

Flow:

```text
Shortcut (Meta+Z / Win+Z) → speak → Shortcut → Gemini 3.5 Transcribe Live → focused field
```

The UI stays toggle-based: press once to start recording, press again to finish, then the final transcription is typed directly into the focused field in one shot. Audio is streamed to Gemini while recording, but interim text is never typed on screen.

**The clipboard is never read or modified.**

---

## Desktop & OS Support

AI Dikte uses one direct text-injection backend for the active desktop:

- **Windows 10 / 11**: Direct Win32 `SendInput` with `KEYEVENTF_UNICODE` (supports full Unicode, Turkish characters `ç, ğ, ı, ö, ş, ü, İ, Ğ...`, and emojis with zero clipboard usage).
- **KDE Plasma / KWin (Wayland)**: `kwtype`
- **Hyprland / Omarchy (Wayland)**: `wtype`

The Linux installer detects the current desktop and installs **only the backend that desktop needs**. Hyprland/Omarchy does not install Qt/KWayland/KWtype, while KDE does not install `wtype`. There is intentionally no clipboard fallback.

---

## Gemini Transcription

The app uses `gemini-3.5-transcribe-live` with:

- Turkish language hint: `tr-TR` (customizable)
- `SMART` transcription mode
- manual activity boundaries matching the two-press toggle workflow
- optional custom vocabulary for names and technical terms

Google's Live Transcription API receives raw 16-bit PCM mono audio at 16 kHz. Finalized speech segments are preserved and combined before the completed transcription is injected into the active field.

---

## Installation

### Windows (10 / 11)

#### One-Line PowerShell Installer (Recommended)

Open PowerShell and run:

```powershell
irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex
```

The installer downloads the latest runtime-tested standalone `ai-dikte-windows.exe` from GitHub Releases, installs it under `%LOCALAPPDATA%\Programs\AI-Dikte`, puts the `ai-dikte` launcher first in the user PATH, creates a hidden Startup launcher, then runs `setup` and `doctor`. Python and pip are not required for the normal Windows installation.

If GitHub Releases is temporarily unavailable, the installer can fall back to a Python/source installation when Python is already installed.

#### Standalone Executable (.exe)

Every successful push to `main` refreshes the **Latest Windows Build** GitHub Release with a newly built and runtime-tested `ai-dikte-windows.exe`. Tags matching `v*` additionally create normal versioned releases.

The Windows CI validates both the packaged version command and a frozen-runtime self-test that imports the actual Windows dependencies (`websockets`, `sounddevice`, `pystray`, and Pillow), checks the fixed `Win+Z` configuration, and verifies the direct `SendInput` backend before publishing the binary.

---

### Linux (Arch / CachyOS / Omarchy / KDE)

#### One-Line Installer (Recommended, Git-Free)

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.sh)"
```

The installer detects the active desktop before installing anything extra:

- **Hyprland / Omarchy** → installs `wtype` only.
- **KDE Plasma Wayland / CachyOS KDE** → builds the pinned `KWtype` backend as the separate `ai-dikte-kwtype` package. Its build-only dependencies are removed automatically after the build.

The base `ai-dikte` package itself contains no bundled typing backend and depends only on the common runtime pieces used by dictation.

#### Manual Installation

Hyprland / Omarchy:

```bash
git clone https://github.com/Yakrel/ai-dikte.git
cd ai-dikte
sudo pacman -S --needed base-devel wtype
makepkg -si
ai-dikte setup
ai-dikte doctor
```

KDE Plasma Wayland:

```bash
git clone https://github.com/Yakrel/ai-dikte.git
cd ai-dikte
sudo pacman -S --needed base-devel
makepkg -p PKGBUILD.kwtype -sric --needed --asdeps
makepkg -si
ai-dikte setup
ai-dikte doctor
```

`setup` stores the Gemini API key and, when Hyprland is detected, installs a managed `Meta+Z` binding. KDE Plasma gets the same `Meta+Z` shortcut through the installed KGlobalAccel desktop entry.

---

## Configuration

Persistent configuration file:

- **Windows**: `%APPDATA%\ai-dikte\config.json` (e.g. `C:\Users\<User>\AppData\Roaming\ai-dikte\config.json`)
- **Linux**: `~/.config/ai-dikte/config.json`

Example configuration:

```json
{
  "api_key": "YOUR_GEMINI_API_KEY",
  "language": "tr-TR",
  "mode": "SMART",
  "hotkey": "win+z",
  "custom_vocabulary": [
    "Proxmox",
    "Omarchy",
    "Hyprland"
  ],
  "output_driver": "auto"
}
```

- `mode`: `SMART` or `VERBATIM`.
- `output_driver`: `auto`, `sendinput` (Windows), `kwtype` / `wtype` (Linux).
- `hotkey`: fixed to `win+z` on Windows. Linux desktop bindings remain `Meta+Z`.
- `custom_vocabulary`: supports up to 1000 domain-specific terms.

---

## Usage

- **Windows**: Runs in background daemon mode with a System Tray icon. Press `Win+Z` once to start recording, speak, and press it again to finish.
- **Linux**: Press `Meta+Z` once to start recording, speak, and press it again to finish.

### Commands

```bash
ai-dikte toggle            # Toggle dictation (start/stop)
ai-dikte daemon            # Run background hotkey listener & system tray
ai-dikte setup             # Configure API key and preferences
ai-dikte doctor            # Run diagnostic checks
ai-dikte shortcut-install  # Install Hyprland shortcut (Linux)
ai-dikte shortcut-remove   # Remove Hyprland shortcut (Linux)
```

Windows standalone diagnostics also support:

```powershell
ai-dikte --version
ai-dikte --self-test
```

---

## CI Maintenance

GitHub Actions artifacts are retained for 7 days. A scheduled cleanup workflow runs daily and keeps only the newest 5 completed runs for each workflow, so old build logs and artifacts do not accumulate indefinitely.

---

## Uninstallation

### Windows

1. Delete `%LOCALAPPDATA%\Programs\AI-Dikte` (or `%LOCALAPPDATA%\ai-dikte` for a Python fallback installation) and `%APPDATA%\ai-dikte`.
2. Remove `ai-dikte-startup.vbs` from your Startup folder (`Win+R` → `shell:startup`).
3. Remove the AI Dikte install directory from your User PATH if desired.

### Linux

On Hyprland / Omarchy:

```bash
ai-dikte shortcut-remove
sudo pacman -Rns ai-dikte
rm -rf ~/.config/ai-dikte
```

On KDE Plasma:

```bash
sudo pacman -Rns ai-dikte ai-dikte-kwtype
rm -rf ~/.config/ai-dikte
```
