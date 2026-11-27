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
  # locking mycelium alone; behind with that one open it does nothing; a
  # newer upstream opens the newer and closes the older; and a fetch that
  # fails ends it non-zero, its line written.
  lock-follows-opens-one-pull-request-per-rev =
    pkgs.runCommandLocal "lock-follows-opens-one-pull-request-per-rev"
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
  # background, catching up on a quarter slept through, for mycelium alone
  # unless it names more.
  lock-follows-runs-every-quarter-hour =
    let
      followed = account (bareNixos [
        self.nixosModules.workstation
        { home-manager.users.someone.imports = [ self.homeModules.lock-follows ]; }
      ]);
      service = followed.systemd.user.services.lock-follows.Service;
      timer = followed.systemd.user.timers.lock-follows.Timer;
    in
    holds "lock-follows-runs-every-quarter-hour"
      "lock-follows runs other than every 15 minutes, misses a quarter slept through, runs outside background.slice, or follows more than mycelium by default."
      (
        timer.OnCalendar == "*:0/15"
        && timer.Persistent
        && service.Slice == "background.slice"
        && lib.hasSuffix " -- mycelium" service.ExecStart
      );
}
