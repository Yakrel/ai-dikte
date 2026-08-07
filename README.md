# Gemini Dictation

Minimal KDE Plasma Wayland dictation for NixOS using PipeWire + Gemini REST API.

Flow: `pw-record` → Gemini → `wl-copy` → `ydotool`.

The `main` branch is NixOS-first. The previous CachyOS/Arch version is kept in the `cachyos` branch.

## NixOS

Add the repository as a flake input:

```nix
ai-dikte = {
  url = "git+ssh://git@github.com/Yakrel/ai-dikte.git";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

Import and enable the module:

```nix
modules = [
  ai-dikte.nixosModules.default
  {
    programs.gemini-dikte = {
      enable = true;
      user = "YOUR_USERNAME";
    };
  }
];
```

The module:

- installs Gemini Dictation;
- uses NixOS' built-in `programs.ydotool` service;
- grants the selected user access to ydotool;
- expects PipeWire to already be enabled.

It does not create a venv, install pip packages, use the `input` group, run `modprobe`, or modify KDE configuration.

`wl-copy` and `notify-send` are available only to the wrapped application and are not added separately to the normal system PATH.

## Setup

```bash
gemini-dikte setup
gemini-dikte doctor
```

The only persistent application file is:

```text
~/.config/gemini-dikte/config.json
```

It contains the API key and settings with mode `0600`.

Recording PID/audio/temporary stderr are stored under `$XDG_RUNTIME_DIR/gemini-dikte` (normally `/run/user/<uid>/gemini-dikte`) and are not persistent.

## KDE shortcut

Create a custom shortcut for:

```text
/run/current-system/sw/bin/gemini-dikte-toggle.sh
```

Recommended: `Meta+Z`.

Notifications use KDE's normal notification system through `notify-send`.

## Remove cleanly

To also remove the API key/config:

```bash
gemini-dikte-purge
```

Then remove the flake/module configuration and rebuild NixOS. Remove the KDE shortcut manually if you created it.

No binaries are copied into `~/.local/bin`. Old `/nix/store` paths remain only while referenced by old NixOS generations and are handled by normal Nix garbage collection.

## Commands

```bash
gemini-dikte toggle
gemini-dikte start
gemini-dikte stop
gemini-dikte cancel
gemini-dikte status
gemini-dikte doctor
gemini-dikte config-path
```
