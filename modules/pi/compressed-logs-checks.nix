# The checks of the nightly rewrite of pi's closed session logs, which
# pi's checks.nix gathers beside its own, kept apart since they change with
# compressed-logs.nix and the script it runs.
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
  # The rewrite runs each night in the background, catching up on a night
  # the machine was off, over pi's sessions folder alone and only where
  # there is one, with no hard clock under three hours; a Mac gets none.
  pi-keeps-closed-logs-compressed =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-logs-compressed;
      timer = home.systemd.user.timers.pi-logs-compressed;
      first = value: lib.head (lib.toList value);
      sessions = "${home.home.homeDirectory}/.pi/agent/sessions";
    in
    holds "pi-keeps-closed-logs-compressed"
      "pi's closed session logs aren't rewritten compressed each night, catching up a night missed while off, run outside background.slice, over a folder other than pi's sessions, where there is none, with an I/O class of their own or a clock under three hours, or reach a Mac."
      (
        lib.elem "timers.target" timer.Install.WantedBy
        && timer.Timer.Persistent
        && lib.hasPrefix "*-*-* " (first timer.Timer.OnCalendar)
        && service.Service.Type == "oneshot"
        && service.Service.Slice == "background.slice"
        && service.Service.TimeoutStartSec == "3h"
        && !(service.Service ? IOSchedulingClass)
        && lib.hasSuffix "/bin/pi-compress-logs ${sessions}" (first service.Service.ExecStart)
        && first service.Unit.ConditionPathIsDirectory == sessions
        && !(mac.systemd.user.services ? pi-logs-compressed)
        && !(mac.systemd.user.timers ? pi-logs-compressed)
      );

  # Off btrfs the rewrite ends at once, with nothing done and saying so,
  # rather than failing every night on a machine whose file system doesn't
  # compress. /proc is never btrfs, in a build's sandbox or out of it.
  pi-log-compression-passes-over-other-file-systems =
    let
      home = account workstation;
      service = home.systemd.user.services.pi-logs-compressed;
      program = lib.head (lib.splitString " " (lib.head (lib.toList service.Service.ExecStart)));
    in
    pkgs.runCommandLocal "pi-log-compression-passes-over-other-file-systems" { } ''
      said=$(${program} /proc)
      echo "$said"
      case $said in
        *"isn't on btrfs, so there's nothing to compress") touch $out ;;
        *) echo "the rewrite didn't say it passed over /proc" >&2; exit 1 ;;
      esac
    '';
}
