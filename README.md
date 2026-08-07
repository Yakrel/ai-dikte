# Gemini Dictation — Minimal v2

A minimal dictation tool for KDE Plasma Wayland **without local speech models**.

Flow:

1. Press the KDE shortcut; recording starts via `pw-record`.
2. Press the same shortcut again; recording stops.
3. The WAV file is sent to the Gemini REST API as a single HTTP request.
4. Gemini transcribes and cleans up the speech.
5. Text is copied with `wl-copy` and pasted into the active window with `ydotool`.

**No Live API, WebSocket, application daemon, venv, pip package, or local speech model is required.**

## NixOS — recommended

The repository exports both a Nix package and a small NixOS module. The module keeps system integration declarative while leaving the application in this standalone repository.

### Flake input

While this repository is private, use an SSH Git input and make sure the machine evaluating the flake can authenticate to GitHub:

```nix
ai-dikte = {
  url = "git+ssh://git@github.com/Yakrel/ai-dikte.git";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

If the repository becomes public later, this can be simplified to:

```nix
ai-dikte = {
  url = "github:Yakrel/ai-dikte";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

Add `ai-dikte` to the flake outputs arguments, then import and enable the module:

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

The module intentionally does only the required system integration:

- installs the `gemini-dikte` package;
- enables NixOS' built-in `programs.ydotool` service/module;
- adds the configured user to the module's `ydotool` group;
- requires an already-enabled PipeWire service for `pw-record`.

It does **not** replace or reconfigure the audio stack, create an `input` group dependency, run `modprobe`, create a Python environment, or modify KDE configuration files.

The package itself provides Python and keeps the two application-specific command dependencies (`wl-copy` from `wl-clipboard` and `notify-send` from `libnotify`) inside the wrapped application's runtime PATH instead of adding them to the user's global PATH.

After rebuilding, log out and back in once so the new `ydotool` group membership and session environment are active. Then configure the API key:

```bash
gemini-dikte setup
gemini-dikte doctor
```

### KDE Plasma shortcut

The NixOS module deliberately does not mutate Plasma shortcut state. Create the shortcut once in:

**System Settings → Keyboard → Shortcuts → Add New → Command or Script**

Use this stable NixOS path:

```text
/run/current-system/sw/bin/gemini-dikte-toggle.sh
```

Recommended shortcut:

```text
Meta+Z
```

The `.sh` wrapper is kept because Plasma 6 custom command shortcuts can be unreliable with extensionless scripts on some systems.

### Clean removal on NixOS

If you also want to remove the API key, log and runtime state, run this **before** removing the package:

```bash
gemini-dikte-purge
```

Then remove the `ai-dikte` input/module configuration and rebuild NixOS. Remove the KDE shortcut if you created it manually.

Nix does not leave copied binaries under `~/.local/bin`; the package and `ydotool` service disappear from the active system generation when the module is removed. Store paths can remain temporarily only while old NixOS generations still reference them, and are removed later by normal Nix garbage collection.

## Installation — CachyOS / Arch

```bash
./install.sh
```

Installation steps:

- Checks/installs `pipewire-audio`, `wl-clipboard`, `ydotool`, and `libnotify` dependencies.
- Installs the main binary to `~/.local/bin/gemini-dikte`.
- Automatically creates KDE shortcut wrapper `~/.local/bin/gemini-dikte-toggle.sh`.
- Adds the user to `input` group if needed for the Arch/CachyOS `ydotool` setup.
- Prompts for the Gemini API key.

API key and settings are saved with `0600` permissions at:

```text
~/.config/gemini-dikte/config.json
```

Default model:

```text
gemini-3.5-flash-lite
```

## KDE Plasma Shortcut — CachyOS / Arch

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

## Terminal testing

```bash
gemini-dikte doctor

gemini-dikte start
# speak
gemini-dikte stop
```

To test the shortcut wrapper directly:

```bash
gemini-dikte-toggle.sh
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
```

On NixOS:

```bash
systemctl status ydotoold.service
```

On CachyOS / Arch:

```bash
systemctl --user status ydotool.service
```

## Uninstallation — CachyOS / Arch

```bash
./uninstall.sh
```

Also remove the custom shortcut created in KDE System Settings.
