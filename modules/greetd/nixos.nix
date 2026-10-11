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
  gpu = config.jitsusama.gpu.unit;
in
{
  imports = [ ../gpu/nixos.nix ];

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

    # nixpkgs starts greetd once every other boot job has finished, which held
    # the XPS 14's desktop back by up to 5.6 s, for jobs it doesn't need, such
    # as waiting for the network. What niri does need is the GPU, so where the
    # hardware names it, greetd waits for that instead. Wants rather than
    # Requires: a GPU that never comes up delays the login, not blocks it.
    systemd.services.greetd = lib.mkIf (gpu != null) {
      wants = [ gpu ];
      after = [ gpu ];
      serviceConfig.Type = lib.mkForce "simple";
    };
  };
}
