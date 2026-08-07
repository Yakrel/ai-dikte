{
  description = "Minimal KDE/Wayland dictation using Gemini";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
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
          gemini-dikte = pkgs.callPackage ./nix/package.nix { };
          default = self.packages.${system}.gemini-dikte;
        });

      checks = forAllSystems (system: {
        package = self.packages.${system}.gemini-dikte;
      });

      nixosModules.default = import ./nix/module.nix { inherit self; };
    };
}
