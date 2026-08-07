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
  version = "1";

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

    wrapProgram "$out/bin/gemini-dikte" \
      --prefix PATH : ${lib.makeBinPath [ wl-clipboard libnotify ]}

    cat > "$out/bin/gemini-dikte-toggle.sh" <<'EOF'
    #!/usr/bin/env sh
    SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
    exec "$SCRIPT_DIR/gemini-dikte" toggle
    EOF
    chmod 0755 "$out/bin/gemini-dikte-toggle.sh"
    patchShebangs "$out/bin/gemini-dikte-toggle.sh"

    mkdir -p "$out/share/applications" "$out/share/kglobalaccel"

    cat > "$out/share/applications/gemini-dikte.desktop" <<EOF
    [Desktop Entry]
    Type=Application
    Name=Gemini Dictation
    Comment=Toggle Gemini voice dictation
    Exec=$out/bin/gemini-dikte-toggle.sh
    TryExec=$out/bin/gemini-dikte-toggle.sh
    Terminal=false
    StartupNotify=false
    NoDisplay=true
    X-KDE-Shortcuts=Meta+Z
    EOF

    cp "$out/share/applications/gemini-dikte.desktop" \
      "$out/share/kglobalaccel/gemini-dikte.desktop"

    runHook postInstall
  '';

  meta = {
    description = "Minimal NixOS KDE/Wayland dictation using Gemini";
    mainProgram = "gemini-dikte";
    platforms = lib.platforms.linux;
  };
}
