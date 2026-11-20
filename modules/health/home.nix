# What the machine said about itself today, kept for Joel and for cadence:
# health, a program of core's own (src/main.rs), follows every journal the
# account may read from the day's start, for warnings and graver, for the
# restarts systemd schedules and for the devices' events the kernel says
# only as information, times each boot and shutdown, counts a lid that
# reads shut over a laptop in use, takes the pain-points detector's ranking
# of how using it felt, reads the agents' sessions' logs for the tools'
# failures and Joel's pained words, watches what pi and the shell hold in
# memory by age, counts the agents that died without parking, and keeps
# the day's tally by kind, thing and signature in
# ~/.local/state/health/<date>.json. `health` shows the three most painful
# interactions, then a line a kind, the worst first; `health <thing>`
# shows what it kept saying. On a Mac it follows the unified log's errors
# and faults in place of the journals, with the sessions' logs, the
# detector and the disk guards as on Linux.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux;
in
{
  home.packages = [ package ];

  systemd.user.services.health = lib.mkIf isLinux {
    Unit.Description = "Keeps the day's tally of what the journals said";
    Service = {
      Type = "simple";
      ExecStart = "${lib.getExe package} follow";
      Environment = [ "PATH=${lib.makeBinPath [ pkgs.systemd ]}" ];
      # Each time it starts, after a fault or a stop, it reads today's
      # journals again from the day's start. It writes the day as it ends,
      # by a follower's end or a stop, so only a kill, a crash or a day it
      # can't write loses what it saw itself since its last write, as the
      # lid and the disk, and the days before it held to try again. A stop
      # just past midnight also loses the old day's journal lines it hadn't
      # read yet, as a start reads only today's.
      Restart = "always";
      RestartSec = 5;
      Slice = "background.slice";
      # The day's replay at each start passes the journals' pages through
      # this unit, as do the sessions' logs it reads: the replay's reads
      # alone charged 2.9 GiB on 19 November, and under this mark 258 MiB
      # at their peak, since a high mark is passed briefly while the kernel
      # reclaims. Past it the kernel takes pages back from this unit rather than from
      # what Joel is using, which slowed that replay from 4.8 s to 8.7 s on
      # a quiet disk; a restart on a busy one takes longer. Health's own
      # peak was 54 MiB that day, and its day's file 570 KiB. No hard
      # limit: a kill past one would lose what health holds unwritten.
      MemoryHigh = "256M";
    };
    Install.WantedBy = [ "default.target" ];
  };

  launchd.agents.health = lib.mkIf isDarwin {
    enable = true;
    config = {
      ProgramArguments = [
        (lib.getExe package)
        "follow"
      ];
      RunAtLoad = true;
      # As on Linux, it reads the day again each time it starts, and loses
      # the same to a fault or a stop; launchd waits 10 s between starts.
      KeepAlive = true;
      # Whatever the account runs comes first, as background.slice does on
      # Linux.
      ProcessType = "Background";
      # Where a Mac keeps an account's logs, there before health is.
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/health.log";
    };
  };
}
