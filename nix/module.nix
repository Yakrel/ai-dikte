{ self }:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.gemini-dikte;
  system = pkgs.stdenv.hostPlatform.system;
in
{
  options.programs.gemini-dikte = {
    enable = lib.mkEnableOption "Gemini Dictation";

    user = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "alice";
      description = ''
        Deprecated compatibility option. It is no longer needed because
        Gemini Dictation types through KWin instead of ydotool.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = config.services.pipewire.enable;
        message = "Gemini Dictation requires services.pipewire.enable = true.";
      }
    ];

    environment.systemPackages = [ self.packages.${system}.gemini-dikte ];
  };
}
