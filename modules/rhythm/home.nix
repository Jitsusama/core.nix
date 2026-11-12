# Learns when Joel stops for the night, from the minutes he's at the
# machine, times only, and keeps a profile of it for the evening's
# wind-down to follow (src/main.rs says how). `rhythm` in a terminal says
# it plainly. It reads swayidle-away's mark of someone at the machine, so
# it comes with the graphical role that brings swayidle.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ package ];

  systemd.user.services.rhythm = {
    Unit = {
      Description = "Learns when Joel stops for the night, from when he's at the machine";
      After = [ "graphical-session.target" ];
    };
    Service = {
      ExecStart = "${lib.getExe package} keep";
      Restart = "on-failure";
      Slice = "background.slice";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
