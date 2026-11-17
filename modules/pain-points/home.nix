# pain-points names, every ten minutes, the interactions with the
# launcher, pi and Chrome that slowed Joel most today (src/main.rs says
# how), in $XDG_STATE_HOME/pain-points/<local day>.jsonl, so the day's
# health can say what slowed him down without anyone watching for it. It
# reads what he typed to pi, and when and how he visited pages in Chrome,
# so it can reach no network and write nowhere but its own state; what he
# typed is never written or sent, and no page is ever read. It's built for
# Linux alone, so it never reads a work laptop's pi logs or Chrome.
#
# `pain-points hear`, its listener, runs every five minutes beside it and
# is the one part that keeps his words: it hears what hurt him in each pi
# session he typed in, keeps each pain in its own state, and sends it as a
# finding to the cell that was working on it through the machine's
# mycelium socket alone (src/hear.rs says how), though a send's words sit
# in mycelium's command line while it runs. It reads the asks and
# misses ledgers and never writes them, never sends to the void, and
# sends nothing until routes.json, beside its state, names who it sends
# as.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  realm = config.home.sessionVariables.MYCELIUM_REALM or null;
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
      # Both units share the folder, and systemd sets its mode as each
      # starts, so read keeps it as closed as hear does.
      StateDirectoryMode = "0700";
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

  systemd.user.services.pain-hear = {
    Unit.Description = "Hears what hurt Joel in his pi sessions and sends each pain to its owner";
    Service = {
      Type = "oneshot";
      ExecStart = lib.concatStringsSep " " [
        (lib.getExe package)
        "hear"
        # The account's own mycelium, the one its node and sessions run.
        "--mycelium ${config.home.profileDirectory}/bin/mycelium"
        "--date ${pkgs.coreutils}/bin/date"
      ];
      # Findings go to the realm his sessions are in.
      Environment = lib.optional (realm != null) "MYCELIUM_REALM=${toString realm}";
      Slice = "background.slice";
      StateDirectory = "pain-points";
      # What it keeps holds his own words, so only his account reads it.
      StateDirectoryMode = "0700";
      ProtectHome = "read-only";
      ReadWritePaths = "%S/pain-points";
      # The mycelium's socket is a file under the runtime folder, which a
      # network namespace of its own leaves reachable while keeping it off
      # every network.
      PrivateNetwork = true;
      # A first run reads a day of pi's logs a line at a time; past this,
      # the kernel takes those pages back first.
      MemoryHigh = "256M";
      # A run ends in seconds; one held past this by a node that never
      # answers is stopped before the next is due, five minutes on.
      TimeoutStartSec = "4min";
    };
  };

  systemd.user.timers.pain-hear = {
    Unit.Description = "Hears what hurt Joel in his pi sessions, every five minutes";
    Timer.OnCalendar = "*:0/5";
    Install.WantedBy = [ "timers.target" ];
  };

  systemd.user.timers.pain-points = {
    Unit.Description = "Names what slowed Joel most in the launcher, pi and Chrome, every ten minutes";
    Timer.OnCalendar = "*:0/10";
    Install.WantedBy = [ "timers.target" ];
  };
}
