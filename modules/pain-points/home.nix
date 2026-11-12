# pain-points names, every ten minutes, the interactions with the
# launcher and pi that slowed Joel most today (src/main.rs says how), in
# $XDG_STATE_HOME/pain-points/<local day>.jsonl, so the day's health can
# say what slowed him down without anyone watching for it. It reads what
# he typed to pi, so it can reach no network and write nowhere but its
# own state; what he typed is never written or sent. It's built for
# Linux alone, so it never reads a work laptop's pi logs.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ package ];

  systemd.user.services.pain-points = {
    Unit.Description = "Names the launcher's and pi's interactions that slowed Joel most today";
    Service = {
      Type = "oneshot";
      ExecStart = lib.concatStringsSep " " [
        (lib.getExe package)
        "read"
        "--journalctl ${pkgs.systemd}/bin/journalctl"
        "--date ${pkgs.coreutils}/bin/date"
      ];
      # It reads what's new in a few records each run, and nothing waits
      # on it.
      Slice = "background.slice";
      StateDirectory = "pain-points";
      ProtectHome = "read-only";
      ReadWritePaths = "%S/pain-points";
      # A user unit can't hold IP filters, so IPAddressDeny alone would
      # leave the network open; a network namespace of its own closes it.
      PrivateNetwork = true;
      IPAddressDeny = "any";
    };
  };

  systemd.user.timers.pain-points = {
    Unit.Description = "Names what slowed Joel most in the launcher and pi, every ten minutes";
    Timer.OnCalendar = "*:0/10";
    Install.WantedBy = [ "timers.target" ];
  };
}
