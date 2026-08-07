# Gemini Dictation — Minimal v2

A minimal dictation tool running on KDE Plasma Wayland **without local speech models**.

Flow:

1. Press KDE shortcut; recording starts via `pw-record`.
2. Press the same shortcut again; recording stops.
3. WAV file is sent to the Gemini REST API as a single HTTP request.
4. Gemini transcribes and formats the speech.
5. Text is copied to clipboard via `wl-copy` and pasted into the active window via `ydotool`.

**No Live API, WebSocket, daemon, application service, venv, or third-party Python packages required.**

## Installation — CachyOS / Arch

```bash
./install.sh
```

Installation steps:

- Checks/installs `pipewire-audio`, `wl-clipboard`, `ydotool`, and `libnotify` dependencies.
- Installs main binary to `~/.local/bin/gemini-dikte`.
- **Automatically creates** KDE shortcut wrapper `~/.local/bin/gemini-dikte-toggle.sh`.
- Adds user to `input` group if needed for `ydotool`.
- Prompts for Gemini API key.

API key and settings are saved with `0600` permissions at:

```text
~/.config/gemini-dikte/config.json
```

Default model:

```text
gemini-3.5-flash-lite
```

## KDE Plasma Shortcut — Important

In KDE Plasma 6 on some systems, extensionless scripts/commands under `Command or Script` global shortcuts may silently fail to execute. Therefore, the installer creates a small `.sh` wrapper.

In KDE:

**System Settings → Keyboard → Shortcuts → Add New → Command or Script**

Set command to:

```text
/home/YOUR_USERNAME/.local/bin/gemini-dikte-toggle.sh
```

Recommended shortcut:

```text
Meta+Z
```

Use the wrapper `.sh` file above instead of pointing directly to `/home/.../.local/bin/gemini-dikte toggle`.

## Terminal Testing

```bash
gemini-dikte doctor

gemini-dikte start
# speak
gemini-dikte stop
```

To test the wrapper directly:

```bash
~/.local/bin/gemini-dikte-toggle.sh
```

First run starts recording, second run stops recording and sends audio to Gemini.

Other commands:

```bash
gemini-dikte toggle
gemini-dikte cancel
gemini-dikte status
gemini-dikte setup
gemini-dikte config-path
gemini-dikte log-path
```

## Configuration

```text
~/.config/gemini-dikte/config.json
```

Model, language, prompt, auto-paste, and notifications can be configured here.

## Troubleshooting

```bash
gemini-dikte doctor
gemini-dikte status
tail -n 50 ~/.local/state/gemini-dikte.log
systemctl --user status ydotool.service
```

If the installer added you to the `input` group, you may need to log out and log back into your KDE session for auto-paste to work.

## Uninstallation

```bash
./uninstall.sh
```

Also remove the custom shortcut created in KDE System Settings.
