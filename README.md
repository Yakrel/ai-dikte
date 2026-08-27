# Gemini Dictation

Minimal Wayland dictation for Arch Linux, CachyOS, KDE Plasma, Hyprland, and Omarchy.

Flow:

```text
Meta+Z → speak → Meta+Z → Gemini 3.5 Transcribe Live → focused field
```

The UI stays toggle-based: press once to start recording, press again to finish, then the final transcription is typed into the focused field in one shot. Audio is streamed to Gemini while recording, but interim text is never typed on screen.

The clipboard is never read or modified.

## Desktop support

Gemini Dictation automatically selects a direct text-injection backend:

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

```bash
git clone https://github.com/Yakrel/ai-dikte.git
cd ai-dikte
makepkg -si
gemini-dikte setup
gemini-dikte doctor
```

`setup` stores the Gemini API key and, when Hyprland is detected, installs a managed `Meta+Z` binding using the active Hyprland configuration format:

- Omarchy 4 / Quattro: `~/.config/hypr/bindings.lua`
- legacy Omarchy / generic Hyprland `.conf` setups: `~/.config/hypr/bindings.conf`

If a managed block from an older Gemini Dictation version exists in the other format, setup removes that old block before installing the current one. KDE Plasma gets the same `Meta+Z` shortcut through the installed KGlobalAccel desktop entry.

## Configuration

The persistent application file is:

```text
~/.config/gemini-dikte/config.json
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

Runtime state and diagnostic files live under `$XDG_RUNTIME_DIR/gemini-dikte` and are not persistent across normal reboots. No audio recording file is written to disk during normal use.

## Use

Press `Meta+Z` once to start recording. Press it again to stop. Gemini finalizes the transcription and the result is typed directly into the currently focused field.

Commands:

```bash
gemini-dikte toggle
gemini-dikte setup
gemini-dikte doctor
gemini-dikte shortcut-install
gemini-dikte shortcut-remove
```

## Remove

On Hyprland / Omarchy, remove the managed shortcut first:

```bash
gemini-dikte shortcut-remove
sudo pacman -Rns gemini-dikte
rm -rf ~/.config/gemini-dikte
```

On KDE Plasma, package removal also removes the installed desktop/KGlobalAccel entry.

The known-good NixOS version remains available as the immutable `nixos-v0.2.0` tag. To work from it later:

```bash
git switch -c nixos nixos-v0.2.0
```
