{
  lib,
  stdenvNoCC,
  makeWrapper,
  python3,
  wl-clipboard,
  libnotify,
}:

stdenvNoCC.mkDerivation {
  pname = "gemini-dikte";
  version = "3";

  src = ../gemini-dikte;
  dontUnpack = true;
  dontConfigure = true;
  dontBuild = true;

  nativeBuildInputs = [
    makeWrapper
    python3
  ];

  installPhase = ''
    runHook preInstall

    install -Dm755 "$src" "$out/bin/gemini-dikte"
    patchShebangs "$out/bin/gemini-dikte"

    # Keep app-only helpers out of the normal system PATH.
    wrapProgram "$out/bin/gemini-dikte" \
      --prefix PATH : ${lib.makeBinPath [ wl-clipboard libnotify ]}

    cat > "$out/bin/gemini-dikte-toggle.sh" <<'EOF'
    #!/usr/bin/env sh
    SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
    exec "$SCRIPT_DIR/gemini-dikte" toggle
    EOF
    chmod 0755 "$out/bin/gemini-dikte-toggle.sh"
    patchShebangs "$out/bin/gemini-dikte-toggle.sh"

    cat > "$out/bin/gemini-dikte-purge" <<'EOF'
    #!/usr/bin/env sh
    set -eu

    SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
    "$SCRIPT_DIR/gemini-dikte" cancel >/dev/null 2>&1 || true

    rm -rf "''${XDG_CONFIG_HOME:-$HOME/.config}/gemini-dikte"
    rm -rf "''${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/gemini-dikte"

    printf '%s\n' "[OK] Gemini Dictation config/runtime removed."
    EOF
    chmod 0755 "$out/bin/gemini-dikte-purge"
    patchShebangs "$out/bin/gemini-dikte-purge"

    runHook postInstall
  '';

  meta = {
    description = "Minimal NixOS KDE/Wayland dictation using the Gemini REST API";
    mainProgram = "gemini-dikte";
    platforms = lib.platforms.linux;
  };
}
