# The checks of lock-follows, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
onLinux {
  # lock-follows runs against a bare origin, an upstream git reaches in
  # GitHub's place, and stand-ins for gh and nix (lock-follows-test.sh):
  # up to date it does nothing; behind it opens one draft pull request
  # locking mycelium alone, each upstream subject in a code span so a
  # "Fixes #12" closes nothing; behind with that one open it does
  # nothing; a newer upstream, after main moved on, opens the newer and
  # replaces the older draft, naming it, commenting, closing, then
  # deleting its branch; the newest replaces none of an older whose lock
  # commit someone amended (another input's rev, or its message), one
  # marked ready, another input's or a fork's, keeping each branch; a rev
  # whose draft was closed by hand isn't opened again or pushed over, nor
  # once its branch is deleted; a branch of its name holding the tool's
  # commit with someone's file amended in is left as taken; a failed
  # create ends it non-zero, and the next run opens on the branch already
  # pushed, a fork's closed draft of that name declining nothing; a
  # failed close leaves the older open, its branch kept; a refused delete
  # leaves the olders closed, their branches kept, and says so; and a
  # failed listing or fetch ends it with 75, its line written.
  lock-follows-opens-one-change-per-rev =
    pkgs.runCommandLocal "lock-follows-opens-one-change-per-rev"
      {
        nativeBuildInputs = [
          pkgs.bash
          pkgs.git
          pkgs.jq
        ];
      }
      ''
        mkdir work
        cd work
        bash ${./lock-follows-test.sh} ${lib.getExe (pkgs.callPackage ./package.nix { })}
        touch $out
      '';

  # An account importing lock-follows runs it every 15 minutes in the
  # background, catching up on a quarter slept through, stopped after 10
  # minutes, for mycelium alone unless it names more; one naming no input
  # is refused.
  lock-follows-runs-every-quarter-hour =
    let
      importing =
        settings:
        account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone.imports = [
              self.homeModules.lock-follows
              settings
            ];
          }
        ]);
      followed = importing { };
      service = followed.systemd.user.services.lock-follows.Service;
      timer = followed.systemd.user.timers.lock-follows.Timer;
      refused =
        home:
        lib.any (
          check: !check.assertion && lib.hasPrefix "jitsusama.lock-follows.inputs" check.message
        ) home.assertions;
    in
    holds "lock-follows-runs-every-quarter-hour"
      "lock-follows runs other than every 15 minutes, misses a quarter slept through, runs outside background.slice, runs past 10 minutes, follows more than mycelium by default, or takes no input unrefused."
      (
        timer.OnCalendar == "*:0/15"
        && timer.Persistent
        && service.Slice == "background.slice"
        && service.TimeoutStartSec == "10min"
        && lib.all (lib.hasSuffix " -- mycelium") (lib.toList service.ExecStart)
        && !refused followed
        && refused (importing {
          jitsusama.lock-follows.inputs = [ ];
        })
      );
}
