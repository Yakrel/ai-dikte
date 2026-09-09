{
  description = "AI Dikte for KDE Plasma and Hyprland on NixOS";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      inherit (pkgs) lib;

      kwtype = pkgs.stdenv.mkDerivation {
        pname = "kwtype";
        version = "0.1.0";

        src = pkgs.fetchurl {
          url = "https://github.com/Sporif/KWtype/archive/ac2c3864aaacc31afc252d88d1d4b669270f2f44.tar.gz";
          sha256 = "ec6f1fa5128835dabbbb3f819ba5aea318eb6e81a8ec47d9fb2868af152c7b41";
        };

        strictDeps = true;
        nativeBuildInputs = [
          pkgs.meson
          pkgs.ninja
          pkgs.pkg-config
          pkgs.kdePackages.qtbase
        ];
        buildInputs = [
          pkgs.kdePackages.qtbase
          pkgs.kdePackages.kwayland
          pkgs.wayland
          pkgs.libxkbcommon
        ];
        # KWtype uses QCoreApplication, not a GUI or Qt platform plugins.
        dontWrapQtApps = true;

        postInstall = ''
          install -Dm644 ../LICENSE "$out/share/licenses/kwtype/LICENSE"
        '';

        meta = {
          description = "Direct keyboard input for KDE Plasma Wayland";
          homepage = "https://github.com/Sporif/KWtype";
          license = lib.licenses.mit;
          platforms = [ system ];
          mainProgram = "kwtype";
        };
      };

      source = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [
          ./rust/Cargo.toml ./rust/Cargo.lock ./rust/build.rs ./rust/src
          ./ai-dikte-toggle ./ai-dikte.desktop ./ai-dikte-settings.desktop
          ./ai-dikte.png ./ai-dikte.ico ./LICENSE ./packaging/linux
        ];
      };
      mkAiDikte = desktop: typingBackend: pkgs.rustPlatform.buildRustPackage {
        pname = "ai-dikte-${desktop}";
        version = "0.5.0";
        src = source;
        cargoRoot = "rust";
        buildAndTestSubdir = "rust";
        cargoLock.lockFile = ./rust/Cargo.lock;
        nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
        buildInputs = [ pkgs.wayland pkgs.libxkbcommon pkgs.libGL ];
        # Nix builds disallow socket creation; the dedicated native CI runs the
        # same localhost WebSocket tests without this sandbox restriction.
        checkFlags = [ "--skip" "live::tests" ];
        postInstall = ''
          wrapProgram "$out/bin/ai-dikte" \
            --prefix PATH : ${lib.makeBinPath [ typingBackend pkgs.pipewire pkgs.libnotify ]} \
            --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [
              pkgs.wayland pkgs.libxkbcommon pkgs.libGL
              pkgs.xorg.libX11 pkgs.xorg.libXcursor pkgs.xorg.libXi pkgs.xorg.libXrandr
            ]}
          install -Dm755 ai-dikte-toggle "$out/bin/ai-dikte-toggle"
          substituteInPlace "$out/bin/ai-dikte-toggle" \
            --replace-fail 'exec ai-dikte toggle' "exec $out/bin/ai-dikte toggle"
          install -Dm644 ai-dikte.png "$out/share/icons/hicolor/256x256/apps/ai-dikte.png"
          install -Dm644 LICENSE "$out/share/licenses/ai-dikte/LICENSE"
          install -Dm644 ai-dikte.desktop "$out/share/applications/ai-dikte.desktop"
          install -Dm644 ai-dikte-settings.desktop "$out/share/applications/ai-dikte-settings.desktop"
          substituteInPlace "$out/share/applications/ai-dikte.desktop" \
            --replace-fail 'Exec=ai-dikte-toggle' "Exec=$out/bin/ai-dikte-toggle"
          substituteInPlace "$out/share/applications/ai-dikte-settings.desktop" \
            --replace-fail 'Exec=ai-dikte' "Exec=$out/bin/ai-dikte"
          install -Dm644 packaging/linux/ai-dikte.service "$out/lib/systemd/user/ai-dikte.service"
          substituteInPlace "$out/lib/systemd/user/ai-dikte.service" \
            --replace-fail '/usr/bin/ai-dikte' "$out/bin/ai-dikte"
          ${lib.optionalString (desktop == "kde") ''
            install -Dm644 "$out/share/applications/ai-dikte.desktop" "$out/share/kglobalaccel/ai-dikte.desktop"
          ''}
        '';
        meta = {
          description = "Rust voice dictation for ${desktop} using Gemini Live";
          homepage = "https://github.com/Yakrel/ai-dikte";
          license = lib.licenses.mit;
          platforms = [ system ];
          mainProgram = "ai-dikte";
        };
      };
    in
    {
      packages.${system} = rec {
        inherit kwtype;
        ai-dikte-kde = mkAiDikte "kde" kwtype;
        ai-dikte-hyprland = mkAiDikte "hyprland" pkgs.wtype;
        default = ai-dikte-kde;
      };
    };
}
