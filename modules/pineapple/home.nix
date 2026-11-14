# The safe word that stops Joel's agents, read where it's said: Slack for
# now, through the harness's Slack sign-in, only read, and in shadow, so
# its stops are written down and nothing acts on them yet (src/lib.rs says
# how the pieces fit). A machine signed in to Slack as Joel imports it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  pineapple = pkgs.callPackage ./package.nix { };
  credentials = "${config.xdg.configHome}/agentic-harness/slack/credentials.json";
in
{
  home.packages = [ pineapple ];

  # Started once pi has signed in to Slack: with no sign-in there is
  # nothing to read with.
  systemd.user.paths.pineapple-slack = {
    Unit.Description = "Starts reading Slack for the safe word once pi has signed in";
    Path.PathExists = credentials;
    Install.WantedBy = [ "default.target" ];
  };

  systemd.user.services.pineapple-slack = {
    Unit = {
      Description = "Reads Slack for the safe word that stops Joel's agents, in shadow";
      # Restarted after a switch reloads the units, it comes back as the
      # new one.
      X-SwitchMethod = "restart";
    };
    Service = {
      Slice = "background.slice";
      # Shown in ps by its name, not its store path (docs/daemons.md).
      ExecStart = "@${lib.getExe pineapple} pineapple-slack run";
      Restart = "on-failure";
      RestartSec = 30;
    };
  };
}
