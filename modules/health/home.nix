# What the machine said about itself today, kept for Joel and for cadence:
# health, a program of core's own (src/main.rs), follows every journal the
# account may read from the day's start, for warnings and graver, for the
# restarts systemd schedules and for the devices' events the kernel says
# only as information, times each boot and shutdown, counts a lid that
# reads shut over a laptop in use, and keeps the day's tally by kind, thing
# and signature in ~/.local/state/health/<date>.json. `health` shows it a
# line a kind, the worst first; `health <thing>` shows what it kept saying.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ package ];

  systemd.user.services.health = {
    Unit.Description = "Keeps the day's tally of what the journals said";
    Service = {
      Type = "simple";
      ExecStart = "${lib.getExe package} follow";
      Environment = [ "PATH=${lib.makeBinPath [ pkgs.systemd ]}" ];
      # It only ever stops on a fault, and reads the day again from its
      # start when it's back, so nothing is lost.
      Restart = "always";
      RestartSec = 5;
      Slice = "background.slice";
    };
    Install.WantedBy = [ "default.target" ];
  };
}
