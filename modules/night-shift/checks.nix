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
  # itself runs on, and its failure fails the run. A start outside its
  # hours, or a wait for the turn they outlast, runs nothing. An eased bar
  # holds only while Joel is away: not while he's marked there, seen
  # lately, on a call, or on a call it can't tell about, nor with no
  # minutes to say when he left, and his coming back brings the job's own
  # bar back.
  night-shift-stops-for-joel = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in night-shift, its tests included.
  night-shift-passes-clippy = passesClippy "night-shift-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A declared job is tried only at night, never caught up on by day,
  # starts only within its hours however it was started (a job naming its
  # own tries keeps only hours it names, and hours that hold no minute are
  # refused), runs behind everything the account runs, with the limits it
  # names on its own cgroup rather than its watcher's, and stops by the
  # slices' bars; it waits for the turn only when told to, and a wait it
  # was let isn't a failed unit; an account declaring none gets no unit,
  # but still night-shift on its path, for the turn every heavy job by day
  # takes.
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
          home-manager.users.someone.jitsusama.night-shift.jobs.waits = {
            description = "Waits for the turn";
            command = [ "true" ];
            waitsAtMost = 1800;
            runsAtMost = 5400;
            # Well above the window and the wait, as its doc asks.
            longest = "3h";
          };
          home-manager.users.someone.jitsusama.night-shift.jobs.noon = {
            description = "Runs at noon, on tries of its own";
            command = [ "true" ];
            tries = [ "*-*-* 12:00" ];
          };
          home-manager.users.someone.jitsusama.night-shift.jobs.late = {
            description = "Runs late, in hours of its own";
            command = [ "true" ];
            tries = [ "*-*-* 23:00" ];
            hours = "22:00-02:00";
          };
        }
      ]);
      none = account (bareNixos [ self.nixosModules.workstation ]);
      service = declared.systemd.user.services.night-shift-repack.Service;
      unit = declared.systemd.user.services.night-shift-repack.Unit;
      timer = declared.systemd.user.timers.night-shift-repack.Timer;
      says = text: pkgs.lib.hasInfix text (toString service.ExecStart);
      waits = declared.systemd.user.services.night-shift-waits.Service;
      # A job in these hours, its words forced so a refusal shows.
      inHours =
        hours:
        let
          words =
            toString
              (account (bareNixos [
                self.nixosModules.workstation
                {
                  home-manager.users.someone.jitsusama.night-shift.jobs.timed = {
                    description = "Runs in hours";
                    command = [ "true" ];
                    inherit hours;
                  };
                }
              ])).systemd.user.services.night-shift-timed.Service.ExecStart;
        in
        builtins.deepSeq words words;
      saysOf =
        job: text:
        pkgs.lib.hasInfix text (
          toString declared.systemd.user.services."night-shift-${job}".Service.ExecStart
        );
    in
    holds "night-shift-runs-only-at-night"
      "A night job is tried by day or caught up on after a night slept through, runs by day when a switch starts it, keeps hours its own tries didn't name, drops hours it named, takes hours that hold no minute, runs outside background.slice or ahead of the account's work, drops a limit it names or puts it on its watcher too, is restarted by a switch, doesn't stop for the apps', the desktop's or agents' slices, passes its command's % to systemd unescaped, waits for the turn unasked, fails its unit for a wait it was let, names a window of running it wasn't given, loses its unit's limit set beside that window, or an account declaring no job gets one."
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
        && says "--hours 01:30-05:15"
        && saysOf "waits" "--hours 01:30-05:15"
        && !(saysOf "noon" "--hours")
        && saysOf "late" "--hours 22:00-02:00"
        && pkgs.lib.all (wrong: !(builtins.tryEval (inHours wrong)).success) [
          "01:30-01:30"
          "24:00-05:00"
        ]
        && says "--start-under 3"
        && says "--stop app.slice=1"
        && says "--stop session.slice=1"
        && says "--stop agents.slice=10"
        && says "--stop-machine 25"
        && says "-- git maintenance run --task=50%%"
        && !(says "--wait-at-most")
        && !(says "--runs-at-most")
        && pkgs.lib.hasInfix "--runs-at-most 5400 " (toString waits.ExecStart)
        && waits.RuntimeMaxSec == "3h"
        && !(service ? SuccessExitStatus)
        && pkgs.lib.hasInfix "--wait-at-most 1800 " (toString waits.ExecStart)
        && waits.SuccessExitStatus == 75
        && !(none.systemd.user.services ? night-shift-repack)
        && !(none.systemd.user.timers ? night-shift-repack)
        && pkgs.lib.any (package: package.pname or "" == "night-shift") none.home.packages
        && !(says "--away")
      );

  # A job easing its bars while Joel is away tells he is from
  # swayidle-away's mark, rhythm's minutes and a call on this machine,
  # their paths left to systemd to expand, and gives each eased bar; one
  # easing a slice it has no stop for is refused.
  night-shift-eases-its-bars-only-while-joel-is-away =
    let
      withJob =
        job:
        account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone.jitsusama.night-shift.jobs.eased = {
              description = "Eases its bars while Joel is away";
              command = [ "true" ];
            }
            // job;
          }
        ]);
      declared = withJob {
        stopsWhileAway."app.slice" = 10;
        stopMachineWhileAway = 12;
      };
      wrong = withJob { stopsWhileAway."agents-only.slice" = 10; };
      words = toString declared.systemd.user.services.night-shift-eased.Service.ExecStart;
      says = text: pkgs.lib.hasInfix text words;
      # night-shift's own, apart from what the rest of the account asserts.
      ours =
        user: builtins.filter (rule: pkgs.lib.hasPrefix "night-shift job " rule.message) user.assertions;
    in
    holds "night-shift-eases-its-bars-only-while-joel-is-away"
      "A job easing its bars while Joel is away escapes the specifiers in its mark's or minutes' paths, asks about calls beyond this machine or none at all, drops an eased bar or the wait before one, or eases a slice it has no stop for."
      (
        says "--stop app.slice=1 "
        && says "--away-mark %t/someone-at-the-machine "
        && says "--away-seen %S/rhythm/seen "
        && says "--away-after 1800 "
        && builtins.match ".*--away-call '/nix/store/[^ ]*/bin/a-call-is-on --here'.*" words != null
        && says "--away-stop app.slice=10 "
        && says "--away-stop-machine 12 "
        && says " -- true"
        && pkgs.lib.any (rule: !rule.assertion) (ours wrong)
        && pkgs.lib.all (rule: rule.assertion) (ours declared)
        && ours declared != [ ]
      );
}
