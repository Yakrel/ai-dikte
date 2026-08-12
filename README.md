# Gemini Dictation

Minimal KDE Plasma Wayland dictation for CachyOS and Arch Linux.

Flow: `pw-record` → Gemini → `kwtype` → focused field.

The clipboard is never read or modified.

## Install on CachyOS

```bash
git clone https://github.com/Yakrel/ai-dikte.git
cd ai-dikte
makepkg -si
gemini-dikte setup
gemini-dikte doctor
```

The only persistent application file is:

```text
~/.config/gemini-dikte/config.json
```

It contains only the Gemini API key and is mode `0600`.

Temporary recording files live under `$XDG_RUNTIME_DIR/gemini-dikte` and are not persistent. Recorded audio is sent to the Google Gemini API for transcription and is removed locally after the transcription attempt finishes.

## Use

The package installs the KDE Plasma global shortcut:

```text
Meta+Z
```

Press once to start recording and again to transcribe and type the result directly into the focused field.

Direct typing is KDE Plasma Wayland-specific because KWtype uses KWin's Fake Input protocol. If direct typing cannot be initialized, Gemini Dictation reports an error instead of falling back to the clipboard.

## Remove

```bash
sudo pacman -Rns gemini-dikte
rm -rf ~/.config/gemini-dikte
```

The known-good NixOS version remains available as the immutable
`nixos-v0.2.0` tag. To work from it later:

```bash
git switch -c nixos nixos-v0.2.0
```

## Commands

```bash
gemini-dikte toggle
gemini-dikte setup
gemini-dikte doctor
```
