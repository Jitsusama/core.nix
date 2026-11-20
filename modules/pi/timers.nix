# The harness's own scripts that Linux runs on a schedule or at login,
# each from where pi-packages installs the harness: its daily measure, the
# list of sessions a boot's end took away, the windows joined to sessions
# elsewhere, and the sweep of ended sessions' work trees. Each is skipped,
# not failed, on a machine without the harness or before its first
# install, and a Mac, with no systemd, gets none.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # Where pi-packages keeps the harness's current version, and the scripts
  # in it these run.
  scripts = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts";
  measure = "${scripts}/measure.ts";
  restoreFile = "${scripts}/restore-file.ts";
  windowsElsewhere = "${scripts}/windows-elsewhere.ts";
  sweepScript = "${scripts}/pi-sweep.ts";
in
{
  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    # The harness's measure: its cost audit of the last day against the day
    # before, and its learning sensor over the last seven days day by day,
    # as one report in its state folder's measure/. A day rather than a
    # week, so a harness change shows the day after it lands. The run reads
    # the day's session logs, so it waits behind whatever the account is
    # doing.
    systemd.user.services.pi-measure = {
      Unit = {
        Description = "Writes the pi harness's daily measure";
        ConditionPathExists = measure;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe pkgs.nodejs} ${measure}";
        Slice = "background.slice";
      };
    };
    systemd.user.timers.pi-measure = {
      Unit.Description = "Writes the pi harness's measure every morning";
      Timer = {
        OnCalendar = "*-*-* 06:00:00";
        RandomizedDelaySec = "10min";
        # A morning slept through runs once the machine wakes.
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # The launcher's list of the sessions the last boot's end took away,
    # written as the account's session starts, before any pi has: each pi
    # rewrites it as it starts, but until one does the file would still
    # describe an older boot. It reads only the harness's session records.
    systemd.user.services.pi-restore-file = {
      Unit = {
        Description = "Writes which pi sessions the last boot's end took away";
        ConditionPathExists = restoreFile;
      };
      Service = {
        Type = "oneshot";
        # The harness's package.json names no module type, on purpose: two
        # of its modules load with require. Node warns about that on every
        # run, which said nothing at each switch.
        ExecStart = "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${restoreFile}";
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };

    # A session another machine holds runs on through this machine's
    # power-off, so what the boot takes away is only the window here
    # joined to it, and only this machine knows it was here. Every five
    # minutes the harness notes those windows, from kitty's and the
    # mycelium node's answers, so the next boot's list can offer them back.
    # Each minute it cost a node of 160 to 185 MB and 0.6 to 0.8 s of CPU,
    # 60 runs an hour that nearly always found nothing new (80ECO2,
    # 2026-11-12); a window joined in the five minutes before a power-off
    # is the one it misses.
    systemd.user.services.pi-windows-elsewhere = {
      Unit = {
        Description = "Notes the windows joined to sessions other machines hold";
        ConditionPathExists = windowsElsewhere;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${windowsElsewhere}";
        Slice = "background.slice";
      };
    };
    systemd.user.timers.pi-windows-elsewhere = {
      Unit.Description = "Notes every five minutes the windows joined to sessions other machines hold";
      Timer = {
        OnCalendar = "*:0/5";
        AccuracySec = "10s";
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # The sweep of ended sessions' work trees, in a process of its own
    # rather than on a pi host's one thread, where its git calls and folder
    # listings held every conversation in the host for up to 262 s on 19
    # November. It stamps as it starts and as it ends, and while that stamp
    # is under two hours old the hosts' own sweeps stand down, so stopping
    # this timer hands sweeping back to them. It runs in jobs.slice, beneath
    # everything Joel uses, an hour after each sweep ends, however long
    # that sweep took.
    systemd.user.services.pi-sweep = {
      Unit = {
        Description = "Sweeps ended pi sessions' work trees";
        ConditionPathExists = sweepScript;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${sweepScript}";
        Slice = "jobs.slice";
        # A sweep still going after two hours is wedged. Its lock goes stale
        # an hour after it stops, for the next sweep to take.
        TimeoutStartSec = "2h";
      };
    };
    systemd.user.timers.pi-sweep = {
      Unit.Description = "Sweeps ended pi sessions' work trees every hour";
      Timer = {
        OnActiveSec = "5min";
        OnUnitInactiveSec = "1h";
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
