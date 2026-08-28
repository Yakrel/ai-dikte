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

AI Dikte automatically selects a direct text-injection backend:

- **Windows 10 / 11**: Direct Win32 `SendInput` with `KEYEVENTF_UNICODE` (supports full Unicode, Turkish characters `ç, ğ, ı, ö, ş, ü, İ, Ğ...`, and emojis with zero clipboard usage).
- **KDE Plasma / KWin (Wayland)**: `kwtype`
- **Hyprland / Omarchy (Wayland)**: `wtype`

In `auto` mode it follows the current desktop/OS and injects directly into the active window. There is intentionally no clipboard fallback.

---

## Gemini Transcription

The app uses `gemini-3.5-transcribe-live` with:

- Turkish language hint: `tr-TR` (customizable)
- `SMART` transcription mode
- manual activity boundaries matching the two-press toggle workflow
- optional custom vocabulary for names and technical terms

Google's Live Transcription API receives raw 16-bit PCM mono audio at 16 kHz. Only the finalized transcript is injected into the active field.

---

## Installation

### Windows (10 / 11)

#### One-Line PowerShell Installer (Recommended)

Open PowerShell and run:

```powershell
irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex
```

The Windows installer prefers the latest standalone `ai-dikte-windows.exe` from GitHub Releases. In the normal release path, Python and pip are not required. It installs the executable under `%LOCALAPPDATA%\Programs\AI-Dikte`, adds an `ai-dikte` launcher to the user PATH, configures startup, then runs `setup` and `doctor`.

If no tagged standalone release exists yet, the installer temporarily falls back to the Python/source installation path so development builds remain installable.

#### Standalone Executable (.exe)

Tagged releases publish `ai-dikte-windows.exe` as a GitHub Release asset. Every Windows CI build also uploads the executable as a GitHub Actions artifact and smoke-tests the frozen binary before upload.

---

### Linux (Arch / CachyOS / Omarchy / KDE)

#### One-Line Installer (Recommended, Git-Free)

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.sh)"
```

#### Manual Installation

```bash
git clone https://github.com/Yakrel/ai-dikte.git
cd ai-dikte
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
- `hotkey`: `win+z`, `alt+z`, `ctrl+alt+z`, etc. (used by Windows background daemon).
- `custom_vocabulary`: supports up to 1000 domain-specific terms.

---

## Usage

- **Windows**: Runs in background daemon mode with a System Tray icon. Press `Win+Z` (or your configured hotkey) once to start recording, speak, and press it again to finish.
- **Linux**: Press `Meta+Z` once to start recording, speak, and press it again to finish.

### Commands

```bash
ai-dikte toggle            # Toggle dictation (start/stop)
ai-dikte daemon            # Run background hotkey listener & system tray
ai-dikte setup             # Configure API key, hotkey, and preferences
ai-dikte doctor            # Run diagnostic checks
ai-dikte --version         # Print packaged version (also used by CI smoke test)
ai-dikte shortcut-install  # Install Hyprland shortcut (Linux)
ai-dikte shortcut-remove   # Remove Hyprland shortcut (Linux)
```

---

## Uninstallation

### Windows

1. Delete `%LOCALAPPDATA%\Programs\AI-Dikte` (or `%LOCALAPPDATA%\ai-dikte` for a Python fallback installation) and `%APPDATA%\ai-dikte`.
2. Remove `ai-dikte-startup.cmd` from your Startup folder (`Win+R` -> `shell:startup`).
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
sudo pacman -Rns ai-dikte
rm -rf ~/.config/ai-dikte
```
