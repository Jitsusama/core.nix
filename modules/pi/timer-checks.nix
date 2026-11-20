# The checks of the harness's scripts that Linux runs on a schedule or at
# login, which pi's checks.nix gathers beside its own, kept apart since they
# change with timers.nix and nothing else.
{
  account,
  holds,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  workstation,
  ...
}:
# They read the Linux workstation's own units, which a Mac can't evaluate.
onLinux {
  # The harness's measure runs each morning in the background, catching up
  # on a morning slept through, with the store's node, and only where the
  # harness is installed; a Mac, with no systemd, gets none.
  pi-measures-each-day-in-the-background =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-measure;
      timer = home.systemd.user.timers.pi-measure.Timer;
      # home-manager keeps a unit's settings as lists.
      first = value: lib.head (lib.toList value);
    in
    holds "pi-measures-each-day-in-the-background"
      "pi's measure runs outside background.slice, not every day, misses a morning slept through, runs a node off the PATH, fails where the harness isn't installed, or reaches a Mac."
      (
        service.Service.Slice == "background.slice"
        && timer.Persistent
        && lib.hasPrefix "*-*-* " (first timer.OnCalendar)
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} " (first service.Service.ExecStart)
        && lib.hasSuffix "/scripts/measure.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.services ? pi-measure)
      );

  # The launcher's list of what the last boot's end took away is written as
  # the account's session starts, with the store's node, only where the
  # harness is installed; a Mac, with no systemd, gets none.
  pi-writes-what-the-last-boot-left =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-restore-file;
      first = value: lib.head (lib.toList value);
    in
    holds "pi-writes-what-the-last-boot-left"
      "pi's list of the sessions the last boot's end took away isn't written as the session starts, runs outside background.slice or a node off the PATH, logs node's module-type warning, fails where the harness isn't installed, or reaches a Mac."
      (
        lib.elem "default.target" service.Install.WantedBy
        && service.Service.Slice == "background.slice"
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON " (
          first service.Service.ExecStart
        )
        && lib.hasSuffix "/scripts/restore-file.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.services ? pi-restore-file)
      );

  # The windows here joined to another machine's session are noted every
  # five minutes, quietly in the background, only where the harness is installed,
  # so the next boot can offer them back; a Mac gets none.
  pi-notes-windows-on-sessions-elsewhere =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-windows-elsewhere;
      timer = home.systemd.user.timers.pi-windows-elsewhere;
      first = value: lib.head (lib.toList value);
    in
    holds "pi-notes-windows-on-sessions-elsewhere"
      "pi doesn't note every five minutes the windows joined to sessions other machines hold, runs outside background.slice or a node off the PATH, logs node's module-type warning, fails where the harness isn't installed, or reaches a Mac."
      (
        lib.elem "timers.target" timer.Install.WantedBy
        && first timer.Timer.OnCalendar == "*:0/5"
        && service.Service.Slice == "background.slice"
        && lib.hasPrefix "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON " (
          first service.Service.ExecStart
        )
        && lib.hasSuffix "/scripts/windows-elsewhere.ts" (first service.Unit.ConditionPathExists)
        && !(mac.systemd.user.timers ? pi-windows-elsewhere)
      );

  # Ended sessions' work trees are swept in a unit of their own, beneath
  # everything Joel uses, at the top of each hour, rather than on a pi
  # host's one thread; a Mac, with no systemd, gets none yet. That the
  # hosts stand down after it is the harness's to prove, in its tests.
  pi-sweeps-work-trees-in-a-unit-of-its-own =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-sweep;
      timer = home.systemd.user.timers.pi-sweep;
      first = value: lib.head (lib.toList value);
      script = first service.Unit.ConditionPathExists;
      environment = lib.toList service.Service.Environment;
    in
    holds "pi-sweeps-work-trees-in-a-unit-of-its-own"
      "pi's tree sweep doesn't run at the top of each hour by the wall clock and once on waking from a sleep that took one, runs outside jobs.slice or a node off the PATH, runs a script other than the one it checks for, logs node's module-type warning, fails where the harness isn't installed, is stopped by a switch or by its own clock before two hours, finds no git from the store first on its PATH, leaves node and the loader's variables that load code to the user manager, names provider modules no machine gave it, or reaches a Mac."
      (
        lib.elem "timers.target" timer.Install.WantedBy
        && first timer.Timer.OnCalendar == "hourly"
        && timer.Timer.Persistent == true
        && !(timer.Timer ? OnUnitInactiveSec)
        && service.Service.Slice == "jobs.slice"
        && service.Service.TimeoutStartSec == "2h"
        && service.Unit.X-RestartIfChanged == false
        && lib.hasSuffix "/scripts/pi-sweep.ts" script
        &&
          first service.Service.ExecStart
          == "${lib.getExe pkgs.nodejs} --disable-warning=MODULE_TYPELESS_PACKAGE_JSON ${script}"
        && lib.any (lib.hasPrefix "PATH=${lib.makeBinPath [ pkgs.git ]}:") environment
        && !(lib.any (lib.hasPrefix "PI_SWEEP_PROVIDERS=") environment)
        &&
          lib.sort lib.lessThan (lib.toList service.Service.UnsetEnvironment) == lib.sort lib.lessThan [
            "NODE_OPTIONS"
            "NODE_PATH"
            "NODE_COMPILE_CACHE"
            "LD_PRELOAD"
            "LD_AUDIT"
            "LD_LIBRARY_PATH"
            "OPENSSL_CONF"
            "OPENSSL_MODULES"
          ]
        && !(mac.systemd.user.timers ? pi-sweep)
        && !(mac.systemd.user.services ? pi-sweep)
      );
}
