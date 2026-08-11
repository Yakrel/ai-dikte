{ self }:
{ config, lib, pkgs, ... }:

let
  system = pkgs.stdenv.hostPlatform.system;
in
{
  options.programs.gemini-dikte.enable = lib.mkEnableOption "Gemini Dictation";

  config = lib.mkIf config.programs.gemini-dikte.enable {
    assertions = [
      {
        assertion = config.services.pipewire.enable;
        message = "Gemini Dictation requires services.pipewire.enable = true.";
      }
    ];

    environment.systemPackages = [ self.packages.${system}.gemini-dikte ];
  };
}
