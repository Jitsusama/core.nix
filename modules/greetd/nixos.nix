# greetd starts niri at boot. Unlocking the disk has just proved it's Joel, so
# the machine's own account signs in by itself, once; after logging out,
# tuigreet asks who's there.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  niriSession = "${config.programs.niri.package}/bin/niri-session";
in
{
  options.jitsusama.login.account = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "joel";
    description = "The account that signs in by itself at boot, set per machine.";
  };

  config = {
    services.greetd = {
      enable = true;
      settings = {
        default_session.command = lib.escapeShellArgs [
          (lib.getExe pkgs.tuigreet)
          "--time"
          "--remember"
          "--asterisks"
          "--cmd"
          niriSession
        ];
      }
      // lib.optionalAttrs (config.jitsusama.login.account != null) {
        initial_session = {
          user = config.jitsusama.login.account;
          command = niriSession;
        };
      };
    };
  };
}
