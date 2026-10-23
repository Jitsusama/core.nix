# The service manager itself. systemd reads at most five changes to the mount
# table a second, and starts no mount while it's over that, until the second
# is up. The kernel's own filesystems use the five as the machine starts, so
# /boot, /home and the swap waited a second for them, and the mounts after
# those waited another, and the greeter came two seconds late. Fifty a second
# leaves the dozen or so changes a boot makes well inside the limit, and still
# caps how often a storm of mounts has systemd read the table again.
#
# A shutdown or reboot can't hang. A healthy one stops every service in a few
# seconds, but systemd gives that half an hour before it forces its way on,
# and a program that won't stop holds the screen for all of it. A minute is
# ample, and forcing on still syncs the disks and asks what's left to stop.
# What's left is asked to stop and given 90 seconds, then killed and given 90
# more, and a process stuck in a damaged kernel dies of neither. So the
# hardware watchdog, armed for that last stage, resets the machine 30 seconds
# in rather than ten minutes. The disks are synced by then, and a stall in
# the kernel's own reboot is caught the same way.
#
# With Plymouth on, shutdown-status tells it by keyword when either
# happens, since a splash that just sits there says nothing. The services that
# end a healthy stop leave a mark it reads, ignoring any failure to.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  stopsWithin = "1min";
  ends = [
    "reboot"
    "poweroff"
    "halt"
    "kexec"
  ];
  reached = "/run/shutdown-reached-its-end";
  status = lib.getExe (pkgs.callPackage ./shutdown-status/package.nix { });
in
{
  systemd.managerEnvironment.SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST = "50";

  systemd.settings.Manager.RebootWatchdogSec = "30s";
  systemd.targets = lib.genAttrs ends (_: {
    unitConfig.JobTimeoutSec = stopsWithin;
  });

  systemd.services = lib.mkIf config.boot.plymouth.enable (
    {
      shutdown-status = {
        description = "Tells Plymouth when a shutdown is forced on or held up";
        wantedBy = map (end: "${end}.target") ends;
        unitConfig = {
          DefaultDependencies = false;
          SurviveFinalKillSignal = true;
        };
        environment.PLYMOUTH = "${config.boot.plymouth.package}/bin/plymouth";
        serviceConfig = {
          Type = "exec";
          ExecStart = "${status} ${reached}";
        };
      };
    }
    // lib.genAttrs (map (end: "systemd-${end}") ends) (_: {
      serviceConfig.ExecStartPre = "-${pkgs.coreutils}/bin/touch ${reached}";
    })
  );
}
