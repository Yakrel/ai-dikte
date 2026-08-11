{
  lib,
  stdenv,
  stdenvNoCC,
  makeWrapper,
  python3,
  meson,
  ninja,
  pkg-config,
  qt6,
  kdePackages,
  wayland,
  libxkbcommon,
  libnotify,
  kwtypeSrc,
}:

let
  kwtype = stdenv.mkDerivation {
    pname = "kwtype";
    version = "0.1.0-unstable-2026-04-14";
    src = kwtypeSrc;

    nativeBuildInputs = [
      meson
      ninja
      pkg-config
      qt6.wrapQtAppsHook
    ];

    buildInputs = [
      qt6.qtbase
      kdePackages.kwayland
      wayland
      libxkbcommon
    ];

    meta = {
      description = "Virtual keyboard input tool for KDE Plasma Wayland";
      homepage = "https://github.com/Sporif/KWtype";
      license = lib.licenses.mit;
      mainProgram = "kwtype";
      platforms = lib.platforms.linux;
    };
  };
in
stdenvNoCC.mkDerivation {
  pname = "gemini-dikte";
  version = "0.2.0";

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
      --prefix PATH : ${lib.makeBinPath [ kwtype libnotify ]}

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
    homepage = "https://github.com/Yakrel/ai-dikte";
    license = lib.licenses.mit;
    mainProgram = "gemini-dikte";
    platforms = lib.platforms.linux;
  };
}
