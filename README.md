# AI Dikte

Minimal AI-powered dictation for the platforms I actually use: **Windows 11**, **KDE Plasma Wayland**, and **Omarchy / Hyprland**. Speech is streamed to **Gemini 3.5 Transcribe Live** and the completed transcription is typed directly into the focused field.

```text
Meta+Z / Win+Z → speak → Meta+Z / Win+Z → Gemini → focused field
```

The dictation path never reads or modifies the clipboard. The explicit **Copy Diagnostics** action is the only feature that copies text to the clipboard.

---

## Supported platforms

AI Dikte deliberately keeps a small support matrix:

| Platform | Audio | Text injection | Shortcut |
| --- | --- | --- | --- |
| Windows 11 | SoundDevice | Win32 `SendInput` | `Win+Z` native hook |
| KDE Plasma / KWin Wayland | PipeWire `pw-record` | `kwtype` | `Meta+Z` KDE global shortcut |
| Omarchy / Hyprland | PipeWire `pw-record` | `wtype` | `Meta+Z` Hyprland binding |

### Linux distributions

The **runtime target is the desktop/compositor, not the distribution**. The KDE path is intended for the KDE Plasma Wayland systems I use, including **Fedora KDE, CachyOS KDE, and NixOS KDE**. Omarchy uses the Hyprland path.

The provided one-line Linux installer is currently **Arch-based only** and is intended for **Arch / CachyOS / Omarchy**. Fedora and NixOS remain runtime targets, but their distro-native packaging is intentionally not bundled into this installer.

**Not supported:** GNOME, X11 sessions, and other compositors/desktops. Unsupported sessions fail explicitly instead of trying another backend.

There is no backend fallback:

- KDE Plasma Wayland → `kwtype`
- Hyprland / Omarchy → `wtype`
- Windows → `SendInput`

The backend and hotkey are runtime/platform properties, not user-configurable settings. Old `output_driver` or `hotkey` keys are ignored and removed the next time settings are saved.

---

## Gemini transcription

The app uses `gemini-3.5-transcribe-live` with:

- default dictation language `tr-TR` (configurable),
- `SMART` or `VERBATIM` transcription mode,
- manual activity boundaries matching the two-press toggle workflow,
- optional custom vocabulary for names and technical terms,
- raw 16-bit PCM mono audio at 16 kHz.

Final transcript segments are appended in order, including intentional repetitions. The app waits for completion and pending final text; a timeout or premature disconnect reports an error instead of silently typing an incomplete transcript.

---

## Installation

### Windows 11

Open PowerShell and run:

```powershell
irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex
```

The Windows installer downloads the CI-built `ai-dikte-windows.exe` plus its SHA-256 checksum, verifies both the checksum and frozen-runtime self-test, then installs it under `%LOCALAPPDATA%\Programs\AI-Dikte`.

Download, checksum, self-test, setup, or diagnostic failures stop installation. There is no Python/pip installation fallback and no silent reuse of an old executable.

Every successful push to `main` builds and tests the standalone Windows executable and refreshes the stable **Latest Windows Build** release. Version tags matching `v*` create versioned releases.

### Arch / CachyOS / Omarchy

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.sh)"
```

The installer detects the active supported Wayland desktop before installing its text backend:

- Hyprland / Omarchy → installs `wtype`.
- KDE Plasma Wayland → reuses `kwtype` if present; otherwise builds the pinned `ai-dikte-kwtype` package.

It then installs AI Dikte, opens initial setup, and runs diagnostics. A failed diagnostic is not reported as successful setup.

### Manual Arch-based installation

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

### Fedora KDE / NixOS KDE

The core runtime is intentionally not tied to Arch paths anymore: the installed launcher resolves its sibling `lib/ai-dikte` directory and invokes `python3` through `PATH`, while desktop entries resolve `ai-dikte` / `ai-dikte-toggle` through `PATH` as well.

A distro-native Fedora or NixOS installer/package is not provided here yet. For source testing, install the equivalent dependencies for the distribution and run:

```bash
python ai_dikte.py setup
python ai_dikte.py doctor
python ai_dikte.py toggle
```

KDE still requires `kwtype`, PipeWire, Tk, and the other runtime dependencies. Distro packaging should install the included KDE desktop entry so `Meta+Z` is registered normally.

---

## Configuration

Persistent configuration:

- Windows: `%APPDATA%\ai-dikte\config.json`
- Linux: `~/.config/ai-dikte/config.json`
- Windows API key: stored separately in Windows Credential Manager as `Yakrel/AI-Dikte/GoogleAI`
- Linux API key: stored in the user-only config file

Typical configuration:

```json
{
  "language": "tr-TR",
  "mode": "SMART",
  "custom_vocabulary": [
    "Proxmox",
    "Omarchy",
    "Hyprland"
  ],
  "input_device": null,
  "audio_cue": true,
  "notify_mode": "all"
}
```

- `language`: dictation language code, for example `tr-TR` or `en-US`.
- `mode`: `SMART` or `VERBATIM`.
- `custom_vocabulary`: up to 1000 unique non-empty terms.
- `input_device`: Windows SoundDevice input index; Linux uses the PipeWire system default.
- `audio_cue`: Windows recording sounds.
- `notify_mode`: `all` or `none`; critical errors remain visible.

`output_driver` and `hotkey` are intentionally **not configuration options**. Supported platforms have deterministic backend and shortcut choices.

The shared settings UI is English-only; the dictation language itself remains configurable.

API-key, language, mode, and vocabulary changes are validated against the Live API before saving. Purely local preferences do not require a network validation call.

---

## Usage

- Windows: press `Win+Z` once to start and again to finish.
- KDE Plasma Wayland: press `Meta+Z` once to start and again to finish.
- Omarchy / Hyprland: press `Meta+Z` once to start and again to finish.

Commands:

```bash
ai-dikte toggle            # Start/stop dictation
ai-dikte setup             # Configure API key and preferences
ai-dikte doctor            # Run diagnostics
ai-dikte shortcut-install  # Install managed Hyprland Meta+Z binding
ai-dikte shortcut-remove   # Remove managed Hyprland binding
ai-dikte daemon            # Windows only: tray + Win+Z listener
ai-dikte --version
ai-dikte --self-test
```

Linux uses desktop-managed shortcuts; there is no Linux keyboard-hook daemon.

---

## Runtime structure

- `ai_dikte.py` — single source/frozen entrypoint
- `ai_dikte_core.py` — recording, transcription, sessions, desktop integration
- `ai_dikte_config.py` — config validation and persistence
- `ai_dikte_ui.py` — shared Tk settings/diagnostics UI
- `ai_dikte_win32.py` — Windows Credential Manager, SendInput, Win+Z hook, overlay

The architecture intentionally keeps the main runtime paths explicit:

```text
Windows 11
  SoundDevice → Gemini → SendInput

KDE Plasma Wayland
  pw-record → Gemini → kwtype

Omarchy / Hyprland
  pw-record → Gemini → wtype
```

No text backend automatically switches to another backend after failure.

---

## CI

The Windows workflow runs runtime tests, installer failure-path tests, source/frozen self-tests, builds the standalone executable, generates its checksum, and uploads the artifact.

The Arch workflow runs Python/shell checks, Tk UI tests under Xvfb, builds the base Arch package, runs the installed package self-test, and builds the pinned KDE `kwtype` backend package.

GitHub Actions artifacts are retained for 7 days. Scheduled cleanup keeps only the newest completed workflow runs needed for maintenance.

Real microphone, Wayland shortcut registration, focused-window typing, and a live Gemini session still require a manual smoke test on the actual target desktop.

---

## Uninstallation

### Windows

1. Exit AI Dikte from the tray.
2. Delete `%LOCALAPPDATA%\Programs\AI-Dikte` and `%APPDATA%\ai-dikte`.
3. Remove `AI-Dikte` from Windows Startup if enabled and delete its Start Menu shortcut.
4. Remove `Yakrel/AI-Dikte/GoogleAI` from Windows Credential Manager.

### Hyprland / Omarchy

```bash
ai-dikte shortcut-remove
sudo pacman -Rns ai-dikte
rm -rf ~/.config/ai-dikte
```

Remove `wtype` separately only if nothing else uses it.

### KDE Plasma on Arch-based systems

```bash
sudo pacman -Rns ai-dikte
rm -rf ~/.config/ai-dikte
```

If the installer created `ai-dikte-kwtype`, remove that package too. If you already had another `kwtype` package, keep it if it is used elsewhere.
