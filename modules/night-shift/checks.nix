# The checks of night-shift, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
onLinux {
  # night-shift's tests run as it builds, against a stand-in machine: a
  # quiet one runs a job once a night, a busy one starts nothing, only one
  # job runs at a time, a stall in a watched slice or past the machine's
  # backstop stops it and kills what ignores TERM, a job stalling only
  # itself runs on, and its failure fails the run.
  night-shift-stops-for-joel = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in night-shift, its tests included.
  night-shift-passes-clippy = passesClippy "night-shift-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A declared job is tried only at night, never caught up on by day, runs
  # behind everything the account runs, with the limits it names on its own
  # cgroup rather than its watcher's, and stops by the slices' bars; an
  # account declaring none gets no unit, but still night-shift on its path,
  # for the turn every heavy job by day takes.
  night-shift-runs-only-at-night =
    let
      declared = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone.jitsusama.night-shift.jobs.repack = {
            description = "Repacks a repository";
            command = [
              "git"
              "maintenance"
              "run"
              "--task=50%"
            ];
            memoryHigh = "12G";
            readsAtMost."/dev/nvme0n1" = "50M";
          };
        }
      ]);
      none = account (bareNixos [ self.nixosModules.workstation ]);
      service = declared.systemd.user.services.night-shift-repack.Service;
      unit = declared.systemd.user.services.night-shift-repack.Unit;
      timer = declared.systemd.user.timers.night-shift-repack.Timer;
      says = text: pkgs.lib.hasInfix text (toString service.ExecStart);
    in
    holds "night-shift-runs-only-at-night"
      "A night job is tried by day or caught up on after a night slept through, runs outside background.slice or ahead of the account's work, drops a limit it names or puts it on its watcher too, is restarted by a switch, doesn't stop for the apps', the desktop's or agents' slices, passes its command's % to systemd unescaped, or an account declaring no job gets one."
      (
        timer.OnCalendar == [
          "*-*-* 01:30/15"
          "*-*-* 02..04:00/15"
          "*-*-* 05:00"
        ]
        && !timer.Persistent
        && service.Slice == "background.slice"
        && service.CPUWeight == 1
        && service.IOWeight == 1
        && service.Delegate == "memory io"
        && !(service ? MemoryHigh)
        && !(service ? IOReadBandwidthMax)
        && says "--memory-high 12G"
        && says "--reads-at-most /dev/nvme0n1=50M"
        && unit.X-RestartIfChanged == false
        && says "--start-under 3"
        && says "--stop app.slice=1"
        && says "--stop session.slice=1"
        && says "--stop agents.slice=10"
        && says "--stop-machine 25"
        && says "-- git maintenance run --task=50%%"
        && !(none.systemd.user.services ? night-shift-repack)
        && !(none.systemd.user.timers ? night-shift-repack)
        && pkgs.lib.any (package: package.pname or "" == "night-shift") none.home.packages
      );
}
