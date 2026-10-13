# lisgd turns swipes on the touchscreen into niri's actions, the same as the
# touchpad's: three fingers drag the columns and the workspaces, the way the
# content moves under them, and four up open the overview and four down close
# it. One or two fingers are left to the program under them, since lisgd reads
# the touchscreen beside niri rather than taking it. It runs while the
# desktop does and the touchscreen is there (nixos.nix names it), and stops
# when either goes.
{ lib, pkgs, ... }:
let
  niri = "${lib.getExe pkgs.niri} msg action";

  # How many fingers, the way they move as lisgd spells it (from left to
  # right is LR, from down to up is DU), and niri's action.
  swipes = [
    {
      fingers = 3;
      way = "RL";
      action = "focus-column-right";
    }
    {
      fingers = 3;
      way = "LR";
      action = "focus-column-left";
    }
    {
      fingers = 3;
      way = "DU";
      action = "focus-workspace-down";
    }
    {
      fingers = 3;
      way = "UD";
      action = "focus-workspace-up";
    }
    {
      fingers = 4;
      way = "DU";
      action = "open-overview";
    }
    {
      fingers = 4;
      way = "UD";
      action = "close-overview";
    }
  ];
  # From any edge, of any length, once the fingers lift.
  gesture = swipe: [
    "-g"
    "${toString swipe.fingers},${swipe.way},*,*,R,${niri} ${swipe.action}"
  ];
in
{
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
