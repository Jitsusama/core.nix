# lisgd turns swipes on the touchscreen into niri's actions, the same as the
# touchpad's: three fingers drag the columns and the workspaces, the way the
# content moves under them, and four up open the overview and four down close
# it. One or two fingers are left to the program under them, since lisgd reads
# the touchscreen beside niri rather than taking it. It runs while the
# desktop does and the touchscreen is there (nixos.nix names it), and stops
# when either goes.
#
# The swipes are jitsusama.swipes, which the launcher's shortcuts list beside
# the chords, so what's shown is what lisgd was given.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama) swipes;
  niri = "${lib.getExe pkgs.niri} msg action";

  # The way the fingers move, as lisgd spells it: from down to up is DU.
  ways = {
    up = "DU";
    down = "UD";
    left = "RL";
    right = "LR";
  };
  # From any edge, of any length, once the fingers lift.
  gesture = swipe: [
    "-g"
    "${toString swipe.fingers},${ways.${swipe.way}},*,*,R,${niri} ${swipe.action}"
  ];
in
{
  imports = [ ../keys/home.nix ];

  jitsusama.swipes = [
    {
      fingers = 3;
      way = "left";
      does = "Focus the column to the right";
      action = "focus-column-right";
    }
    {
      fingers = 3;
      way = "right";
      does = "Focus the column to the left";
      action = "focus-column-left";
    }
    {
      fingers = 3;
      way = "up";
      does = "Go to the workspace below";
      action = "focus-workspace-down";
    }
    {
      fingers = 3;
      way = "down";
      does = "Go to the workspace above";
      action = "focus-workspace-up";
    }
    {
      fingers = 4;
      way = "up";
      does = "Show every workspace";
      action = "open-overview";
    }
    {
      fingers = 4;
      way = "down";
      does = "Stop showing every workspace";
      action = "close-overview";
    }
  ];

  systemd.user.services.lisgd = {
    Unit = {
      Description = "Swipes on the touchscreen, as niri's actions";
      ConditionPathExists = "/dev/input/touchscreen";
      BindsTo = [ "dev-input-touchscreen.device" ];
      After = [
        "dev-input-touchscreen.device"
        "graphical-session.target"
      ];
      # It asks the desktop the screen's size, so a touchscreen plugged in
      # before anyone's signed in waits for the desktop to start it.
      Requisite = [ "graphical-session.target" ];
      PartOf = [ "graphical-session.target" ];
    };
    Service = {
      ExecStart = lib.escapeShellArgs (
        [
          (lib.getExe pkgs.lisgd)
          "-d"
          "/dev/input/touchscreen"
        ]
        ++ lib.concatMap gesture swipes
      );
      Restart = "on-failure";
      RestartSec = 1;
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
