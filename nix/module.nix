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
      type = lib.types.str;
      example = "alice";
      description = "Login user allowed to use ydotool for automatic paste.";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = builtins.hasAttr cfg.user config.users.users;
        message = "programs.gemini-dikte.user must name an existing NixOS user.";
      }
      {
        assertion = config.services.pipewire.enable;
        message = "Gemini Dictation requires services.pipewire.enable = true.";
      }
    ];

    environment.systemPackages = [ self.packages.${system}.gemini-dikte ];
    programs.ydotool.enable = true;
    users.users.${cfg.user}.extraGroups = [ config.programs.ydotool.group ];
  };
}
