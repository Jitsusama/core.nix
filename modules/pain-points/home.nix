# pain-points names, every ten minutes, the interactions with the
# launcher, pi and Chrome that slowed Joel most today (src/main.rs says
# how), in $XDG_STATE_HOME/pain-points/<local day>.jsonl, so the day's
# health can say what slowed him down without anyone watching for it. It
# reads what he typed to pi, and when and how he visited pages in Chrome,
# so it can reach no network and write nowhere but its own state; what he
# typed is never written or sent, and no page is ever read. It's built for
# Linux alone, so it never reads a work laptop's pi logs or Chrome.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ package ];

  systemd.user.services.pain-points = {
    Unit.Description = "Names the launcher's, pi's and Chrome's interactions that slowed Joel most today";
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
      # A user unit can't hold IP filters, so a network namespace of its
      # own is what keeps it off the network.
      PrivateNetwork = true;
      # A day's first read passes a gigabyte or more of pi's logs through
      # the page cache, charged to this unit; past this, the kernel takes
      # those pages back first rather than what Joel is using.
      MemoryHigh = "128M";
    };
  };

  systemd.user.timers.pain-points = {
    Unit.Description = "Names what slowed Joel most in the launcher, pi and Chrome, every ten minutes";
    Timer.OnCalendar = "*:0/10";
    Install.WantedBy = [ "timers.target" ];
  };
}
