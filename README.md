# Gemini Dictation

Minimal KDE Plasma Wayland dictation for NixOS.

Flow: `pw-record` → Gemini → `kwtype` → focused field.

The clipboard is never read or modified.

The `main` branch targets NixOS. The previous CachyOS/Arch version is kept in the `cachyos` branch.

## NixOS

Add the flake input:

```nix
ai-dikte = {
  url = "github:Yakrel/ai-dikte";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

Import the module and enable it:

```nix
modules = [
  ai-dikte.nixosModules.default
];

programs.gemini-dikte.enable = true;
```

The module installs Gemini Dictation and expects PipeWire to already be enabled.

The application privately packages [KWtype](https://github.com/Sporif/KWtype) for direct keyboard input through KWin and `notify-send` for desktop notifications. It does not run in the background and does not require `ydotoold` or extra input-group membership.

## Setup

```bash
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

Remove the flake/module configuration and rebuild NixOS. To also delete the API key:

```bash
rm -rf ~/.config/gemini-dikte
```

No files are installed in `~/.local/bin`.

## Commands

```bash
gemini-dikte toggle
gemini-dikte setup
gemini-dikte doctor
```
