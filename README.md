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

The one-line Linux installer remains **Arch-based only** for **Arch / CachyOS / Omarchy**. Fedora KDE has native RPM packages; NixOS has locked KDE and Hyprland flake packages. These use the same runtime and settings UI, not separate distro implementations.

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

Stopping capture flushes audio already delivered by the recorder before ending the API turn. If the microphone delivered no audio, the session fails explicitly; it never substitutes generated silence.

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

### Fedora 44 KDE (x86_64)

Download the `ai-dikte-fedora-44-x86_64` artifact from a successful **Fedora KDE packages** run in [GitHub Actions](https://github.com/Yakrel/ai-dikte/actions/workflows/fedora.yml). It contains the application RPM, the pinned native `kwtype` RPM, and `SHA256SUMS`. Extract it, then run in that directory:

```bash
sha256sum --check SHA256SUMS
sudo dnf install ./kwtype-*.rpm ./ai-dikte-*.rpm
ai-dikte setup
ai-dikte doctor
```

DNF installs the Python, Tk/XWayland, PipeWire and notification dependencies. The RPM includes the KDE `Meta+Z` shortcut entry. This is a local-RPM installation, not a configured package repository: download and install a newer artifact to update.

These are unsigned CI RPMs; verify the checksums from a trusted workflow artifact before installing.

### NixOS (x86_64)

With flakes enabled, install **one** package matching your desktop:

```bash
nix profile install github:Yakrel/ai-dikte#ai-dikte-kde
# Or, on Hyprland:
nix profile install github:Yakrel/ai-dikte#ai-dikte-hyprland
```

For a declarative NixOS configuration, add `inputs.ai-dikte.url = "github:Yakrel/ai-dikte";` to your flake and include the selected package in your module:

```nix
{ inputs, ... }: {
  environment.systemPackages = [
    inputs.ai-dikte.packages.x86_64-linux.ai-dikte-kde
  ];
}
```

Pass `inputs` through your configuration's `specialArgs` as usual. The package supplies its own Python/websockets/Tk and only the selected typing backend; it does not use system Python packages. Your system must still provide the supported Wayland desktop, PipeWire service and XWayland for Tk.

After installing, run `ai-dikte setup` and `ai-dikte doctor`. On Hyprland, install the managed binding with `ai-dikte shortcut-install`; if your compositor configuration is declaratively managed, declare `SUPER, Z, exec, ai-dikte-toggle` there instead. KDE packages include the global shortcut entry.

`flake.lock` pins nixpkgs, and KWtype uses the same pinned upstream revision as the Arch and Fedora packages. The generic Linux launcher also resolves profile symlinks before locating the application's modules.

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
  "audio_cue": false,
  "notify_mode": "all"
}
```

- `language`: dictation language code, for example `tr-TR` or `en-US`.
- `mode`: `SMART` or `VERBATIM`.
- `custom_vocabulary`: up to 1000 unique non-empty terms.
- `input_device`: Windows SoundDevice input index; Linux uses the PipeWire system default.
- `audio_cue`: Windows recording sounds, independent of visual notifications.
- `notify_mode`: visual status notifications, `all` or `none`; critical errors remain visible even when ordinary notifications are off.

`output_driver` and `hotkey` are intentionally **not configuration options**. Supported platforms have deterministic backend and shortcut choices.

The shared settings UI is English-only; the dictation language itself remains configurable.

API-key, language, mode, and vocabulary changes are validated against the Live API before saving. Purely local preferences do not require a network validation call.

On Windows, **Play audio cues** and **Show visual status notifications** are separate switches. **Test sound** plays the same WAV/`PlaySound` path used during dictation without saving settings, contacting Gemini, or requiring an API key. It works even when recording cues are disabled. Playback errors appear in the dialog; if Windows accepts playback but you hear nothing, check the selected Windows output device and the app's volume/mute setting in Volume Mixer. There is no fallback beep or alternate playback backend. During dictation, an audio-feedback failure is logged rather than undoing successful text insertion.

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

The Fedora workflow builds both RPMs from the checked-out source, installs them in Fedora 44, and exercises the installed application outside the checkout before uploading RPMs and checksums.

The Nix workflow builds both locked desktop variants and exercises each installed launcher with no host Python or tools on `PATH`.

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

### Fedora KDE

Run `sudo dnf remove ai-dikte`. Remove `kwtype` separately only if nothing else uses it. Delete `~/.config/ai-dikte` if you also want to remove the configuration and API key.

### NixOS

Remove the package from `environment.systemPackages` and rebuild your system. For a profile install, use `nix profile list` and then `nix profile remove` with the displayed package name. Remove any managed Hyprland binding before uninstalling; declarative bindings should be removed from your compositor configuration. Configuration and the API key remain under `~/.config/ai-dikte` until explicitly deleted.
