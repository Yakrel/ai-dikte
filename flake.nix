{
  description = "Minimal KDE/Wayland dictation using Gemini";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    kwtype-src = {
      url = "github:Sporif/KWtype/ac2c3864aaacc31afc252d88d1d4b669270f2f44";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, kwtype-src }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          gemini-dikte = pkgs.callPackage ./nix/package.nix {
            kwtypeSrc = kwtype-src;
          };
          default = self.packages.${system}.gemini-dikte;
        });

      checks = forAllSystems (system: {
        package = self.packages.${system}.gemini-dikte;
      });

      nixosModules.default = import ./nix/module.nix { inherit self; };
    };
}
