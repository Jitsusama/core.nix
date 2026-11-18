# The checks of update-to-main, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves. Its VM
# test is checks.nix's vm-updates-itself-to-signed-main.
{
  bareDarwin,
  bareNixos,
  holds,
  lib,
  passesClippy,
  pkgs,
  self,
  ...
}:
let
  settings = {
    jitsusama.update-to-main = {
      repository = "https://example.com/machines";
      output = "system";
      askers = [ "someone" ];
    };
  };
  nixos =
    (bareNixos [
      self.nixosModules.update-to-main
      settings
    ]).config;
  darwin =
    (bareDarwin [
      self.darwinModules.update-to-main
      settings
    ]).config;
  service = nixos.systemd.services.update-to-main;
  daemon = darwin.launchd.daemons.update-to-main.serviceConfig;
  followed =
    (bareDarwin [
      self.darwinModules.update-to-main
      settings
      { jitsusama.update-to-main.follower = "someone"; }
    ]).config;
  follows = followed.launchd.user.agents.update-to-main-follows.serviceConfig;
  refusesOnAMac =
    why: lib.filter (refusal: refusal.why == why) followed.jitsusama.update-to-main.refuseWhile;
in
{
  # update-to-main's tests run as it builds: a signed main is built and
  # switched to, main where it was builds nothing, a hosted main whose
  # folder is where it was is neither fetched nor built, an unsigned main, one
  # signed by a key not given and a rewritten one are refused, a failed
  # build leaves the machine as it was, a failed switch and a machine not
  # whole after it go back, a new kernel's userland runs at once and goes
  # back if it isn't whole while the kernel waits for the next boot, a low
  # battery starts nothing, asks during an update join it, anything but the
  # one ask is refused, the socket is only its group's to write, a switch
  # that would restart a held daemon or change a held kernel waits for an
  # asker's grant, and the
  # follower asks when a head moves, soon again after a busy refusal, not
  # again at heads that failed, and never past a held daemon's refusal.
  update-to-main-updates-to-signed-main = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in update-to-main, its tests included.
  update-to-main-passes-clippy = passesClippy "update-to-main-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # A switch restarts what changed, and update-to-main is what's switching,
  # so neither system may restart it mid-switch: NixOS never restarts its
  # service, and the Mac's daemon names no store path, so activate reloads
  # it only when its definition itself changes, and the switch outlives it
  # then, since it writes to a file rather than to the daemon. Both start it
  # from the running system's path, the Mac's once that path holds it, and
  # its askers are its group's members. The commands it refuses while are
  # read too, each word a string: on 8 November one given as a derivation
  # broke every NixOS machine following main, and nothing here read them.
  # Neither config names the running system's path, which a binary from
  # before it would refuse, and a reloaded Mac daemon runs that binary.
  update-to-main-outlives-its-switch =
    holds "update-to-main-outlives-its-switch"
      "A switch could restart update-to-main while it switches, the Mac's daemon doesn't wait for its program, its askers aren't its group's, a command it refuses while isn't strings, or its config names a field an older update-to-main refuses."
      (
        !(nixos.jitsusama.update-to-main.system ? current)
        && !(darwin.jitsusama.update-to-main.system ? current)
        && !service.restartIfChanged
        && !service.stopIfChanged
        && lib.hasPrefix "/run/current-system/sw/bin/update-to-main " service.serviceConfig.ExecStart
        && !(lib.hasInfix "/nix/store" (builtins.toJSON daemon))
        &&
          lib.take 2 daemon.ProgramArguments == [
            "/bin/sh"
            "-c"
          ]
        && lib.hasPrefix "until [ -x /run/current-system/sw/bin/update-to-main ]; do sleep 1; done; exec /run/current-system/sw/bin/update-to-main serve " (
          lib.last daemon.ProgramArguments
        )
        && nixos.users.groups.update-ask.members == [ "someone" ]
        && darwin.users.groups.update-ask.members == [ "someone" ]
        && lib.all (refusal: lib.all builtins.isString refusal.command) (
          nixos.jitsusama.update-to-main.refuseWhile ++ darwin.jitsusama.update-to-main.refuseWhile
        )
        && lib.any (
          refusal:
          refusal.followerWaits
          && refusal.why == "someone is at the machine"
          && refusal.followerWaitsAtMost == 45 * 60
        ) nixos.jitsusama.update-to-main.refuseWhile
      );
  # A Mac's follower is its primary user's launchd agent, started again
  # whenever follow ends, as it does when it can't start a newer one, and
  # naming no store path, so a switch never reloads it under its own ask.
  # Every Mac refuses an update during a call, and refuses the ask alone,
  # for two hours at most, while nix-offload says it's busy; and the
  # follower's switch waits while someone is at the Mac, for 45 minutes at
  # most. When the
  # Mac can't update, the follower tells its person.
  update-to-main-follows-on-a-mac =
    holds "update-to-main-follows-on-a-mac"
      "A Mac's follower wouldn't start again after follow ends, names a store path a switch would reload it for, tells nobody when the Mac can't update, or a Mac would switch during a call or under someone at it, or build while over its room, or let that hold its own switch or every update for good."
      (
        follows.Label == "org.jitsusama.update-to-main-follows"
        && follows.RunAtLoad
        && !follows.KeepAlive.SuccessfulExit
        && !(lib.hasInfix "/nix/store" (builtins.toJSON follows.ProgramArguments))
        && lib.hasSuffix "/run/current-system/sw/bin/update-to-main follow; exit 75" (
          lib.last follows.ProgramArguments
        )
        && !(darwin.launchd.user.agents ? update-to-main-follows)
        && followed.jitsusama.update-to-main.followerTells != null
        && lib.all (refusal: !refusal.followerWaits) (refusesOnAMac "a call is on")
        && refusesOnAMac "a call is on" != [ ]
        && lib.any (
          refusal:
          !refusal.followerWaits
          && refusal.refusesTheAskAlone
          && refusal.refusesAtMost == 2 * 60 * 60
          && lib.hasSuffix "/bin/nix-offload" (lib.head refusal.command)
          && lib.tail refusal.command == [ "busy" ]
        ) (refusesOnAMac "the Mac is over its room")
        && lib.any (refusal: refusal.followerWaits && refusal.followerWaitsAtMost == 45 * 60) (
          refusesOnAMac "someone is at the machine"
        )
      );
  # The Mac's room check names a question nix-offload answers, not words it
  # refuses as asked wrongly, which update-to-main would read as can't tell
  # and refuse all but one ask in two hours for: while this change was
  # written, nix-offload had no busy yet, and the words alone passed above.
  # Built here from the package the Mac names, its words are asked as the
  # daemon asks them, and answer as a question does: 0 busy, 1 room, 69
  # can't say, as on a builder with no sysctl; never 64, asked wrongly.
  update-to-main-asks-what-nix-offload-answers =
    let
      words = lib.tail (lib.head (refusesOnAMac "the Mac is over its room")).command;
      offload = pkgs.callPackage ../nix-offload/offload/package.nix { };
    in
    # Kept in Nix because the command it runs is the Mac's own, read from
    # this evaluation, and the rest only sorts its exit code.
    pkgs.runCommand "update-to-main-asks-what-nix-offload-answers" { } ''
      rc=0
      ${lib.escapeShellArgs ([ (lib.getExe offload) ] ++ words)} || rc=$?
      case $rc in
        0 | 1 | 69) touch $out ;;
        *) echo "nix-offload ${lib.concatStringsSep " " words} answered $rc, not a question's answer" >&2; exit 1 ;;
      esac
    '';
  # launchd gives a job 256 open files unless it says otherwise, and git
  # opens every pack it reads: on 16 November a Mac's update failed with
  # "Could not read" an object, its main's checkout holding about 489
  # packs. So the daemon and the follower each open up to 8192, their hard
  # limit left as launchd's unlimited one, so nothing they start gets less.
  update-to-main-opens-enough-files-on-a-mac =
    holds "update-to-main-opens-enough-files-on-a-mac"
      "A Mac's update-to-main daemon or follower keeps launchd's 256 open files, too few for git to read a checkout of many packs."
      (
        lib.all
          (
            job:
            (job.SoftResourceLimits.NumberOfFiles or 0) >= 8192
            && (job.HardResourceLimits.NumberOfFiles or null) == null
          )
          [
            daemon
            follows
          ]
      );
}
