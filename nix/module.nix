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
      description = "Login user that may use ydotool for automatic paste.";
    };

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${system}.gemini-dikte;
      description = "Gemini Dictation package to install.";
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
        message = "Gemini Dictation requires services.pipewire.enable = true for pw-record.";
      }
    ];

    environment.systemPackages = [ cfg.package ];

    # ydotool's NixOS module owns the daemon, /dev/uinput access, socket and
    # ydotool package. No input group, manual modprobe or user service needed.
    programs.ydotool.enable = true;
    users.users.${cfg.user}.extraGroups = [ config.programs.ydotool.group ];
  };
}
