# Gemini Dictation

Minimal KDE Plasma Wayland dictation for NixOS.

Flow: `pw-record` → Gemini → `wl-copy` → `ydotool` → automatic `Ctrl+Shift+V`.

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

programs.gemini-dikte = {
  enable = true;
  user = "YOUR_USERNAME";
};
```

The module installs Gemini Dictation, enables NixOS' `ydotoold` service, adds the selected user to the ydotool group, and expects PipeWire to already be enabled.

`wl-copy` and `notify-send` are private runtime dependencies of the wrapped application. Gemini Dictation itself does not run in the background.

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

Temporary recording files live under `$XDG_RUNTIME_DIR/gemini-dikte` and are not persistent.

## Use

The package installs the KDE Plasma global shortcut:

```text
Meta+Z
```

Press once to start recording and again to transcribe and paste into the focused field.

After the first installation, log out and back in once so ydotool group membership is applied.

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
