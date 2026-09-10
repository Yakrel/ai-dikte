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
- Installs it to `%LOCALAPPDATA%\Programs\AI-Dikte\ai-dikte.exe` and adds it to your user `PATH`.
- Opens the first-run setup in your terminal to enter and verify your Gemini API key.
- Starts the silent background listener daemon and enables it to run at sign-in.

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

---

## Command Reference

| Command | Description |
|---|---|
| `ai-dikte` / `ai-dikte menu` | Interactive console menu |
| `ai-dikte setup` | First-run setup wizard (API key entry and test) |
| `ai-dikte daemon` | Headless background hotkey listener |
| `ai-dikte toggle` | (Linux) Trigger recording from desktop shortcut |
| `ai-dikte status` | Plain text background service and configuration status |
| `ai-dikte doctor` | Full diagnostics report (microphone, network, Gemini API) |
| `ai-dikte logs` | Print recent session log |
| `ai-dikte --self-test` | Runtime self-test without network or credentials |

---

## Architecture

AI Dikte is built with pure Rust and zero heavy UI dependencies. Complete architectural decisions, invariants, and guidelines are documented in [AGENTS.md](AGENTS.md).
