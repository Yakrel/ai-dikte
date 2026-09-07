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

      python = pkgs.python3.withPackages (ps: [ ps.websockets ps.tkinter ]);
      source = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [
          ./ai_dikte.py
          ./ai_dikte_core.py
          ./ai_dikte_config.py
          ./ai_dikte_ui.py
          ./ai-dikte-toggle
          ./ai-dikte.desktop
          ./ai-dikte-settings.desktop
          ./ai-dikte.png
          ./LICENSE
        ];
      };

      mkAiDikte = desktop: typingBackend: pkgs.stdenvNoCC.mkDerivation {
        pname = "ai-dikte-${desktop}";
        version = "0.4.0";
        src = source;
        nativeBuildInputs = [ pkgs.makeWrapper ];
        dontBuild = true;
        strictDeps = true;

        installPhase = ''
          runHook preInstall

          for module in ai_dikte.py ai_dikte_core.py ai_dikte_config.py ai_dikte_ui.py; do
            install -Dm644 "$module" "$out/lib/ai-dikte/$module"
          done
          install -Dm644 ai-dikte.png "$out/lib/ai-dikte/ai-dikte.png"
          install -Dm644 ai-dikte.png "$out/share/icons/hicolor/256x256/apps/ai-dikte.png"
          install -Dm644 LICENSE "$out/share/licenses/ai-dikte/LICENSE"

          # Generate the relocatable launcher's Nix equivalent with a fixed interpreter.
          # Keep host PATH after packaged tools so session-provided hyprctl stays available.
          makeWrapper ${python}/bin/python3 "$out/bin/ai-dikte" \
            --add-flags "$out/lib/ai-dikte/ai_dikte.py" \
            --prefix PATH : ${lib.makeBinPath [ typingBackend pkgs.pipewire pkgs.libnotify python ]} \
            --set PYTHONNOUSERSITE 1 \
            --unset PYTHONPATH \
            --unset PYTHONHOME
          install -Dm755 ai-dikte-toggle "$out/bin/ai-dikte-toggle"
          substituteInPlace "$out/bin/ai-dikte-toggle" \
            --replace-fail 'exec ai-dikte toggle' "exec $out/bin/ai-dikte toggle"

          install -Dm644 ai-dikte.desktop "$out/share/applications/ai-dikte.desktop"
          install -Dm644 ai-dikte-settings.desktop "$out/share/applications/ai-dikte-settings.desktop"
          substituteInPlace "$out/share/applications/ai-dikte.desktop" \
            --replace-fail 'Exec=ai-dikte-toggle' "Exec=$out/bin/ai-dikte-toggle"
          substituteInPlace "$out/share/applications/ai-dikte-settings.desktop" \
            --replace-fail 'Exec=ai-dikte' "Exec=$out/bin/ai-dikte"
          ${lib.optionalString (desktop == "kde") ''
            install -Dm644 "$out/share/applications/ai-dikte.desktop" \
              "$out/share/kglobalaccel/ai-dikte.desktop"
          ''}

          runHook postInstall
        '';

        meta = {
          description = "Minimal Wayland dictation for ${desktop} using Gemini Transcribe Live";
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
