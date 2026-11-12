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
      # It only ever stops on a fault, and reads the day again from its
      # start when it's back, so nothing is lost.
      Restart = "always";
      RestartSec = 5;
      Slice = "background.slice";
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
      # As on Linux, it only stops on a fault and reads the day again when
      # it's back; launchd waits 10 s between starts.
      KeepAlive = true;
      # Whatever the account runs comes first, as background.slice does on
      # Linux.
      ProcessType = "Background";
      # Where a Mac keeps an account's logs, there before health is.
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/health.log";
    };
  };
}
