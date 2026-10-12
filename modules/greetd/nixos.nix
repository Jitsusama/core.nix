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
  # Prints the name of each variable in the environment that systemd takes as
  # valid, which is every name of ASCII letters, digits and underscores not
  # starting with a digit. gawk lists AWKPATH and AWKLIBPATH whether they're
  # set or not, so they're left out.
  environmentNames = pkgs.writeShellScript "environment-names" ''
    exec ${lib.getExe pkgs.gawk} 'BEGIN {
      for (v in ENVIRON)
        if (v ~ /^[A-Za-z_][A-Za-z0-9_]*$/ && v != "AWKPATH" && v != "AWKLIBPATH")
          print v
    }'
  '';

  # niri's session script imports the login's environment with a bare
  # `systemctl --user import-environment`, which systemd deprecates with a
  # warning that lands on the console as the desktop starts. This passes the
  # same variables by name. A name systemd won't take fails the whole call,
  # hence the filter. Upstream, resources/niri-session's line would become
  # `systemctl --user import-environment $(awk 'BEGIN { for (v in ENVIRON) if
  # (v ~ /^[A-Za-z_][A-Za-z0-9_]*$/ && v != "AWKPATH" && v != "AWKLIBPATH")
  # print v }')`; take this out once niri's does.
  niriSession = pkgs.runCommand "niri-session" { } ''
    substitute ${config.programs.niri.package}/bin/niri-session $out \
      --replace-fail 'systemctl --user import-environment' \
        'systemctl --user import-environment $(${environmentNames})'
    chmod +x $out
  '';

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
