# AI Dikte

Minimal, fast, and local voice dictation utility for Windows and Linux Wayland powered by Google Gemini Live.

Press **Win+Z** (Windows) or **Meta+Z** (Linux) to start talking; press it again to finish. Transcribed text is instantly typed into the active window.

---

## Installation

### Windows

Run this single command in **PowerShell** (no administrator privileges required):

```powershell
irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex
```

*What this does:*
- Downloads the standalone `ai-dikte-windows.exe` and verifies its SHA-256 checksum.
- Installs the single executable to `%LOCALAPPDATA%\Programs\AI-Dikte\ai-dikte.exe` and adds its `bin` command-launcher directory to your user `PATH`.
- Opens the first-run setup in your terminal to enter and verify your Gemini API key.
- Starts the silent background listener daemon and enables it to run at sign-in.
- Waits for the listener to become ready, checks local configuration, then returns to the PowerShell prompt without waiting for the daemon to exit.

`ai-dikte setup` is a one-shot command; `ai-dikte` opens the interactive menu. Setup errors return a nonzero exit code. An unreadable or malformed configuration is reported rather than silently reset.

On Windows, the installed `ai-dikte` command uses a small `.cmd` launcher so PowerShell waits while the menu reads keyboard input. The executable keeps its GUI subsystem so the daemon stays console-free. Use `ai-dikte` from the updated `PATH`, rather than launching `ai-dikte.exe` directly in PowerShell.

If an API key is already stored, setup asks `Replace existing API key? [y/N]:`. Press Enter or answer `n` to keep it without rewriting it. Answer `y` to enter a replacement; the new key is saved only after successful verification. Canceling entry or failing verification leaves the previous key intact.

Interrupting hidden API-key entry with Ctrl+C restores terminal echo and leaves the saved key unchanged.

---

### Linux (KDE Plasma & Hyprland / Omarchy)

#### Arch Linux / CachyOS / Omarchy (One-Line Installer)
```sh
bash -c "$(curl -fsSL https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.sh)"
```

#### Arch Linux / CachyOS (Manual Source Build)
```sh
makepkg --syncdeps --install
ai-dikte setup
systemctl --user enable --now ai-dikte.service
```
*(For KDE, ensure `kwtype` is installed. For Hyprland, ensure `wtype` is installed.)*

#### Fedora (RPM)
```sh
sudo dnf install ./ai-dikte*.rpm
ai-dikte setup
systemctl --user enable --now ai-dikte.service
```

#### NixOS (Flake)
Add `ai-dikte-kde` or `ai-dikte-hyprland` to your NixOS configuration:
```nix
environment.systemPackages = [ ai-dikte-kde ];
systemd.packages = [ ai-dikte-kde ];
```
Then run:
```sh
ai-dikte setup
systemctl --user enable --now ai-dikte.service
```

#### Hotkey Binding on Linux:
- **Hyprland:** Add to `hyprland.conf`:
  ```ini
  bind = $mainMod, Z, exec, ai-dikte toggle
  ```
- **KDE Plasma:** In System Settings -> Custom Shortcuts, bind `Meta+Z` to `ai-dikte toggle`.

---

## How It Works

1. **Hotkey:** Press `Win+Z` (Windows) or `Meta+Z` (Linux) anywhere (VS Code, Word, browser, etc.).
2. **Visual Feedback:**
   - **Windows:** A sleek, non-intrusive floating OSD capsule appears in the bottom-right corner:
     - `● Listening...` (Pink dot) while you talk.
     - `● Transcribing...` (Yellow dot) when you finish.
     - `● Text inserted` (Green dot) when text is typed into the focused window.
   - **Linux:** Desktop notifications appear via `notify-send`.
3. **No Interruption:** No black console windows pop up; no system tray icon clutter.

---

## Settings & Interactive Menu

To manage settings, test your microphone, or view diagnostics, simply run `ai-dikte` in any terminal:

```sh
ai-dikte
```

```text
=====================================================
                   AI DIKTE (v0.5.0)                    
=====================================================
 [Status]     : Background service is RUNNING
 [Hotkey]     : Win+Z (Press to talk, press to finish)
 [Voice/Mode] : tr-TR (Smart)
 [Microphone] : System Default
-----------------------------------------------------
 [1] Enter / Update API Key
 [2] System & Connection Diagnostics (Doctor)
 [3] Writing Style (Smart / Verbatim) & Vocabulary
 [4] Start at Sign-in: [ENABLED]
 [5] Background Service: [Stop]
 [0] Exit
-----------------------------------------------------
 Choice [0-5]: _
```

- **English UI:** The interface is clean, minimal, and in English.
- **Turkish Speech:** Voice recognition defaults to Turkish (`tr-TR`).
- **Safe saves:** Changing the API key and then editing writing settings preserves the new key. Invalid existing configuration must be repaired before setup can overwrite it.
- **Failure handling:** Recorder failures and unsettled transcription timeouts abort text insertion. Stopping the daemon cancels an active session; text already injected into another application cannot be rolled back.
- **Accidental activation:** Empty recordings and audio classified locally as non-speech finish without waiting for Gemini's transcription timeout, including when connection setup is still pending. Speech detection is local; actual dictation is sent to Gemini. Detected speech retains the full finalization timeout so slow responses are not silently discarded.
- **Exact Windows hotkey:** Only Win+Z starts a session; adding Ctrl, Alt, or Shift does not trigger dictation.
- **Hyprland configuration:** Shortcut management preserves symlinks and leaves unchanged files untouched.

The obsolete `audio_cue` setting has been removed. Before upgrading an existing configuration containing it, remove that property from `%APPDATA%\ai-dikte\config.json` on Windows or `$XDG_CONFIG_HOME/ai-dikte/config.json` (normally `~/.config/ai-dikte/config.json`) on Linux. Preserve the other settings and credentials. Unknown properties remain errors; configuration is never silently reset.

---

## Command Reference

| Command | Description |
|---|---|
| `ai-dikte` / `ai-dikte menu` | Interactive console menu |
| `ai-dikte setup` | First-run setup wizard (API key entry and test) |
| `ai-dikte daemon` | Headless background hotkey listener |
| `ai-dikte toggle` | (Linux) Trigger recording from desktop shortcut |
| `ai-dikte status` | Plain text background service and configuration status |
| `ai-dikte doctor` | Local status report; use menu option `[2]` to test the live API connection |
| `ai-dikte logs` | Print recent session log |
| `ai-dikte --self-test` | Runtime self-test without network or credentials |

---

## Architecture

AI Dikte is built with pure Rust and zero heavy UI dependencies. Complete architectural decisions, invariants, and guidelines are documented in [AGENTS.md](AGENTS.md).

Local speech detection uses the small, embedded [Earshot](https://docs.rs/earshot/) model in pure Rust. It processes 16 kHz mono audio without a separate runtime or model download. No detector can perfectly distinguish every noise from quiet speech; short and reduced-volume speech are covered by regressions.

## Verification

Run from `rust/`:

```sh
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

Live tests are opt-in. Set `GEMINI_API_KEY` in the process environment, then run:

```sh
cargo test --locked --test live_api -- --ignored --nocapture --test-threads=1
```

They exercise authentication, accidental empty/noisy recordings, connected silence, and normal, quiet, and short speech. They stream generated PCM directly; they do not capture a microphone or type into another application. Live tests use network access and API quota, and recognition results can vary.

The committed `rust/tests/fixtures/*.pcm` files are signed 16-bit little-endian, 16 kHz mono speech generated with FFmpeg's Flite `slt` voice: “Hello, this is a dictation test.” and “Hello.” They contain no user audio or credentials. Arch source manifests include these fixtures and the CLI integration tests; regenerate with `bash packaging/update-arch-sources.sh` after changing packaged files.
