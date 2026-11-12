# pain-points names, every ten minutes, the interactions with the
# launcher that cost Joel most today (src/main.rs says how), in
# $XDG_STATE_HOME/pain-points/<local day>.jsonl, so the day's health can
# say what slowed him down without anyone watching for it.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ package ];

  systemd.user.services.pain-points = {
    Unit.Description = "Names the launcher's interactions that cost Joel most today";
    Service = {
      Type = "oneshot";
      ExecStart = lib.concatStringsSep " " [
        (lib.getExe package)
        "read"
        "--journalctl ${pkgs.systemd}/bin/journalctl"
        "--date ${pkgs.coreutils}/bin/date"
      ];
      # It reads two small records in a few tens of milliseconds, and
      # nothing waits on it.
      Slice = "background.slice";
    };
  };

  systemd.user.timers.pain-points = {
    Unit.Description = "Names the launcher's interactions that cost Joel most, every ten minutes";
    Timer.OnCalendar = "*:0/10";
    Install.WantedBy = [ "timers.target" ];
  };
}
