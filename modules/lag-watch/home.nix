# lag-watch keeps a standing watch on whether what Joel uses waits
# (src/main.rs says how), so a change that makes the desktop wait is seen
# by its numbers, beside the generation that brought it, rather than
# felt days later. `lag-watch now` says what the last ten seconds were
# like; `lag-watch report` sums up the last day by generation and load.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ package ];

  systemd.user.services.lag-watch = {
    Unit = {
      Description = "Watches whether what Joel uses waits";
      PartOf = [ "graphical-session.target" ];
      After = [ "graphical-session.target" ];
    };
    Service = {
      ExecStart = "${lib.getExe package} run";
      # Beside the compositor and sound, so it isn't starved while it
      # measures what starves them; every ten seconds it reads a small
      # file for each watched thread and each app process, a few dozen,
      # and every two seconds three for each watched thread, its state,
      # its schedstat and its sched, to tell a sleep the kernel booked as
      # a wait.
      Slice = "session.slice";
      Restart = "on-failure";
      RestartSec = "5s";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
