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

The installer does **not** build on the local computer. It downloads the `latest` CI-built `ai-dikte-windows.exe` and its SHA-256 checksum, verifies both the checksum and frozen-runtime self-test, then replaces the installed executable under `%LOCALAPPDATA%\Programs\AI-Dikte`. A failed download never deletes a working verified installation. Python and pip are not required for the normal Windows installation; a Python/source fallback is used only when no verified executable is available.

#### Standalone Executable (.exe)

Every successful **push** to `main`—not a local commit by itself—runs the Windows CI, executes the runtime contract tests, builds the standalone executable, checks the frozen runtime, emits a SHA-256 checksum, and refreshes the stable **Latest Windows Build** release without deleting the release first. Tags matching `v*` additionally create immutable versioned releases.

---

### Linux (Arch / CachyOS / Omarchy / KDE)

#### One-Line Installer (Recommended, Git-Free)

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.sh)"
```

The installer detects the active desktop before installing anything extra:

- **Hyprland / Omarchy** → installs `wtype` only.
- **KDE Plasma Wayland / CachyOS KDE** → reuses `kwtype` if it is already available; otherwise it builds the pinned backend as the separate `ai-dikte-kwtype` package. Build-only dependencies are removed automatically afterward.

The selected backend stays explicitly installed so a generic orphan cleanup cannot silently remove AI Dikte's active typing backend. The base `ai-dikte` package itself contains no bundled typing backend and depends only on the common runtime pieces used by dictation.

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
command -v kwtype >/dev/null || makepkg -p PKGBUILD.kwtype -sric --needed
makepkg -si
ai-dikte setup
ai-dikte doctor
```

`setup` stores the Gemini API key and, when Hyprland is detected, installs a managed `Meta+Z` binding. KDE Plasma gets the same `Meta+Z` shortcut through the installed KGlobalAccel desktop entry.

---

## Configuration

Persistent non-secret configuration:

- **Windows**: `%APPDATA%\ai-dikte\config.json`; the Google AI API key is stored separately as `Yakrel/AI-Dikte/GoogleAI` in the current user's Windows Credential Manager.
- **Linux**: `~/.config/ai-dikte/config.json`; the API key remains in this user-only file.

Example Windows configuration:

```json
{
  "language": "tr-TR",
  "mode": "SMART",
  "hotkey": "win+z",
  "custom_vocabulary": [
    "Proxmox",
    "Omarchy",
    "Hyprland"
  ],
  "output_driver": "auto",
  "input_device": null,
  "audio_cue": true,
  "notify_mode": "all"
}
```

- `mode`: `SMART` or `VERBATIM`.
- `input_device`: `null` for the system default or the numeric SoundDevice input index selected by Setup.
- `audio_cue`: enables Windows start/stop/finish sounds.
- `notify_mode`: `all` or `none`; critical errors remain visible.
- `output_driver`: `auto`, `sendinput` (Windows), `kwtype` / `wtype` (Linux).
- `hotkey`: fixed to `win+z` on Windows. Linux desktop bindings remain `Meta+Z`.
- `custom_vocabulary`: supports up to 1000 unique non-empty terms.

---

## Usage

- **Windows**: Runs in background daemon mode with a System Tray icon. Press `Win+Z` once to start recording, speak, and press it again to finish.
- **Linux**: Press `Meta+Z` once to start recording, speak, and press it again to finish.

On Windows, right-click the tray icon to configure the validated API key, language, mode, microphone, vocabulary, sounds, notifications, and sign-in startup. The same menu exposes Doctor, current microphone/model status, logs, copyable diagnostics, restart, and exit actions.

Setup validates the exact Gemini Live model connection before replacing the working key or saving any preference. A failed key, quota, model-access, or network check leaves the prior configuration intact.

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

1. Exit AI Dikte from the tray, then delete `%LOCALAPPDATA%\Programs\AI-Dikte` (or `%LOCALAPPDATA%\ai-dikte` for a Python fallback installation) and `%APPDATA%\ai-dikte`.
2. Remove `AI-Dikte` from **Windows Settings → Apps → Startup** (or Registry Run key) and delete `AI Dikte.lnk` from your Start Menu.
3. Open Windows Credential Manager → **Windows Credentials** and remove `Yakrel/AI-Dikte/GoogleAI`.
4. Remove the AI Dikte install directory from your User PATH if desired.

### Linux

On Hyprland / Omarchy:

```bash
ai-dikte shortcut-remove
sudo pacman -Rns ai-dikte
rm -rf ~/.config/ai-dikte
```

`wtype` is installed as the selected desktop backend. Remove it separately with `sudo pacman -Rns wtype` only if you do not use it for anything else.

On KDE Plasma:

```bash
sudo pacman -Rns ai-dikte
rm -rf ~/.config/ai-dikte
```

If the installer created `ai-dikte-kwtype`, remove that package too. If you already had another `kwtype` package, leave it installed if you still use it elsewhere.
