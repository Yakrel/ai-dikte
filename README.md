# AI Dikte

Minimal Wayland dictation for Arch Linux, CachyOS, KDE Plasma, Hyprland, and Omarchy using Gemini 3.5 Transcribe Live.

Flow:

```text
Meta+Z → speak → Meta+Z → Gemini 3.5 Transcribe Live → focused field
```

The UI stays toggle-based: press once to start recording, press again to finish, then the final transcription is typed into the focused field in one shot. Audio is streamed to Gemini while recording, but interim text is never typed on screen.

The clipboard is never read or modified.

## Desktop support

AI Dikte automatically selects a direct text-injection backend:

- KDE Plasma / KWin: `kwtype`
- Hyprland / Omarchy: `wtype`

In `auto` mode it follows the current desktop first and can fall back to the other installed direct-typing backend if the preferred one fails. There is intentionally no clipboard fallback.

## Gemini transcription

The app uses `gemini-3.5-transcribe-live` with:

- Turkish language hint: `tr-TR`
- `SMART` transcription mode
- manual activity boundaries matching the two-press toggle workflow
- optional custom vocabulary for names and technical terms

Google's Live Transcription API receives raw 16-bit PCM mono audio at 16 kHz. Only the finalized transcript is injected into the active field.

## Install on Arch / CachyOS / Omarchy

### One-line installer (recommended, git-free)

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.sh)"
```

### Manual installation

```bash
git clone https://github.com/Yakrel/ai-dikte.git
cd ai-dikte
makepkg -si
ai-dikte setup
ai-dikte doctor
```

`setup` stores the Gemini API key and, when Hyprland is detected, installs a managed `Meta+Z` binding using the active Hyprland configuration format:

- Omarchy 4 / Quattro: `~/.config/hypr/bindings.lua`
- legacy Omarchy / generic Hyprland setups: `~/.config/hypr/bindings.conf` or `~/.config/hypr/hyprland.conf`

KDE Plasma gets the same `Meta+Z` shortcut through the installed KGlobalAccel desktop entry.

## Configuration

The persistent application file is:

```text
~/.config/ai-dikte/config.json
```

It is mode `0600`. A typical configuration is:

```json
{
  "api_key": "...",
  "language": "tr-TR",
  "mode": "SMART",
  "custom_vocabulary": [
    "Proxmox",
    "Omarchy",
    "Hyprland"
  ],
  "output_driver": "auto"
}
```

`mode` can be `SMART` or `VERBATIM`. `output_driver` can be `auto`, `kwtype`, or `wtype`. `custom_vocabulary` accepts up to 1000 entries; shorter focused lists are generally preferable.

Runtime state and diagnostic files live under `$XDG_RUNTIME_DIR/ai-dikte` and are not persistent across normal reboots. No audio recording file is written to disk during normal use.

## Use

Press `Meta+Z` once to start recording. Press it again to stop. Gemini finalizes the transcription and the result is typed directly into the currently focused field.

Commands:

```bash
ai-dikte toggle
ai-dikte setup
ai-dikte doctor
ai-dikte shortcut-install
ai-dikte shortcut-remove
```

## Remove

On Hyprland / Omarchy, remove the managed shortcut first:

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
