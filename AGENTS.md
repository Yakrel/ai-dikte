# AI Dikte — Agent & Architecture Guidelines

This document serves as the permanent source of truth for architectural decisions, coding rules, and conventions for AI Dikte. Any agent or developer working on this codebase must adhere strictly to these principles.

---

## 1. Project Overview

AI Dikte is a minimal, fast, and local voice dictation utility powered by the Google Gemini Live WebSocket API. It captures microphone audio, streams it to Gemini Live, and types the transcribed text directly into the active window.

---

## 2. Language Policy

- **Single Language Interface (English Only):**
  - All user interface elements, terminal menus, prompts, help text, log messages, error messages, and OSD text must be in **English**.
  - No dual-language or translated UI strings in the codebase. English provides consistency, simplicity, and avoids localization bloat.
- **Voice Dictation Input (Turkish by Default):**
  - The voice recognition / speech-to-text language is configured to Turkish (`tr-TR`) by default.
  - The user speaks Turkish; the Gemini Live model transcribes Turkish speech into text.

---

## 3. Core Architectural Invariants

### B. Windows Subsystem & Console Management
- The Windows executable is built with `#![cfg_attr(windows, windows_subsystem = "windows")]` (`IMAGE_SUBSYSTEM_WINDOWS_GUI`).
- **Daemon Mode (`ai-dikte daemon`):** Starts completely silently in the background. No console window ever appears; zero flicker.
- **Interactive / CLI Modes (`ai-dikte`, `ai-dikte setup`, `ai-dikte doctor`, etc.):**
  - Terminal-only access: no Start Menu shortcuts are installed. The program is accessed exclusively via command line (`ai-dikte` in PowerShell, CMD, or terminal).
  - Calls `ensure_console()`. If launched from a terminal, it attaches to the parent terminal via `AttachConsole(ATTACH_PARENT_PROCESS)`. If invoked directly, it allocates a console via `AllocConsole()`.

### C. Headless Daemon (No System Tray Icon)
- The Windows daemon operates completely **headless** without a system tray icon (`Shell_NotifyIconW`), tray popup menus, or icon resource management.
- The daemon lifecycle is managed:
  - Through the interactive CLI menu (option `[5]`).
  - Via the Windows named stop event (`Local\AI-Dikte-Daemon-Stop`).
  - On Linux, via the systemd user service (`systemctl --user start/stop ai-dikte.service`).

### D. Minimal Interactive Console Menu (No TUI Bloat)
- Heavy full-screen TUI libraries (e.g. `ratatui`, `crossterm`) are strictly prohibited.
- The user interface is a lightweight, interactive console menu inspired by Massgrave (MAS):
  - Uses standard I/O (`println!`, `read_line`).
  - Uses simple numbered options (`[0-5]`).
  - Never breaks, requires no terminal geometry calculations, and works reliably over SSH, CMD, PowerShell, and Linux ttys.

### E. Zero Audio Playback (No Beep Cues)
- The application is strictly an audio *recording* pipeline (Windows WASAPI, Linux PipeWire).
- All audio playback libraries, WAV files, and cue chimes are removed.

### F. Visual Feedback (HUD / OSD)
- **Windows:** A floating, non-intrusive status capsule at the bottom-right corner of the work area:
  - Created using native Win32 API (`WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`).
  - Styled with a dark theme (`#181825` background, `#313244` border) and bold Segoe UI font.
  - Status indicator dot:
    - `● Listening...` (Pink/Red `#f38ba8`)
    - `● Transcribing...` (Yellow `#f9e2af`)
    - `● Text inserted` (Green `#a6e3a1`, auto-hides after 1.2s)
    - `● Error: [Message]` (Red `#f38ba8`, auto-hides after 3.5s)
- **Linux:** Native desktop notifications via `notify-send` (compatible with KDE Plasma and Hyprland).

### G. Fail-Fast Error Handling
- No silent fallback chains or heuristic guessing.
- Unsupported desktop environments (e.g. non-Wayland on Linux) fail fast with clear diagnostic messages.
- Typing backends are strictly locked: `SendInput` on Windows, `kwtype` on KDE Wayland, and `wtype` on Hyprland.
### H. Zero-Config First-Run Onboarding
- When executed without an existing configuration, `ai-dikte` directly prompts for the Gemini API key.
- The key is validated in real-time with a lightweight WebSocket handshake before saving. Invalid keys are never stored.
- Upon successful verification, the background listener service is started immediately and set to start at sign-in automatically without extra confirmation prompts.
## 4. Testing & Verification

- All Rust tests must be native Rust unit and integration tests executed via `cargo test`.
- Integration tests live in `rust/tests/cli.rs` using Cargo's `env!("CARGO_BIN_EXE_ai-dikte")`.
- No external Python scripts for testing Rust code.
- All code must pass `cargo test --locked`, `cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings`.

---

## 5. Command Reference

| Command | Purpose |
|---|---|
| `ai-dikte` / `ai-dikte menu` | Launches the interactive console menu (or first-run wizard if unconfigured). |
| `ai-dikte setup` | Launches the first-run API key onboarding wizard. |
| `ai-dikte daemon` | Runs the headless background listener daemon. |
| `ai-dikte toggle` | (Linux) Sends start/stop recording signal to the running daemon via Unix socket. |
| `ai-dikte status` | Prints plain text status of the background service and configuration. |
| `ai-dikte doctor` | Runs local environment, microphone, and live API connection diagnostics. |
| `ai-dikte logs` | Prints the session log to terminal. |
| `ai-dikte --self-test` | Runs self-tests without credentials, microphone, or network access. |
