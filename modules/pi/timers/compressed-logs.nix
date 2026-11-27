# pi's session logs, kept compressed on disk. Each is written in small
# appends, which btrfs stores mostly as they came even where the file
# system compresses, so on 23 November only 16% of the old logs' bytes sat
# compressed; rewriting them gave back about 7.9 GB. Each night the logs
# closed since are rewritten the same way, one at a time and behind other
# work, so they stay that way. Nothing that reads a log changes, since its
# bytes and modification time stay as they were. Only Linux runs it, and
# only where pi keeps sessions; off btrfs it finds nothing to do.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  sessions = "${config.home.homeDirectory}/.pi/agent/sessions";
  compress = pkgs.writeShellApplication {
    name = "pi-compress-logs";
    runtimeInputs = [
      pkgs.btrfs-progs
      pkgs.e2fsprogs
      pkgs.coreutils
      pkgs.findutils
      pkgs.gawk
    ];
    text = builtins.readFile ./compress-logs.sh;
  };
in
{
  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    systemd.user.services.pi-logs-compressed = {
      Unit = {
        Description = "Rewrites closed pi session logs compressed on disk";
        ConditionPathIsDirectory = sessions;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe compress} ${sessions}";
        # Beneath what Joel uses. The script waits while other work waits
        # on the disk, so it needs no I/O class of its own, which would only
        # rank it against other background work.
        Slice = "background.slice";
        # The first run over months of logs took about 40 minutes; a
        # night's is minutes. One still going after three hours is wedged.
        TimeoutStartSec = "3h";
      };
    };
    systemd.user.timers.pi-logs-compressed = {
      Unit.Description = "Rewrites closed pi session logs compressed each night";
      Timer = {
        OnCalendar = "*-*-* 04:30:00";
        RandomizedDelaySec = "20min";
        # A night the machine was off runs once it's back.
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
