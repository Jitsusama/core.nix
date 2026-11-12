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
  # service, and the Mac's daemon names no store path, so its definition is
  # the same in every system and activate never reloads it. Both start it
  # from the running system's path, the Mac's once that path holds it, and
  # its askers are its group's members. The commands it refuses while are
  # read too, each word a string: on 8 November one given as a derivation
  # broke every NixOS machine following main, and nothing here read them.
  update-to-main-outlives-its-switch =
    holds "update-to-main-outlives-its-switch"
      "A switch could restart update-to-main while it switches, the Mac's daemon doesn't wait for its program, its askers aren't its group's, or a command it refuses while isn't strings."
      (
        !service.restartIfChanged
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
        && lib.any (
          refusal:
          refusal.followerWaits
          && refusal.why == "the launcher is open"
          && refusal.followerWaitsAtMost == null
        ) nixos.jitsusama.update-to-main.refuseWhile
      );
  # A Mac's follower is its primary user's launchd agent, started again
  # whenever follow ends, as it does for a newer update-to-main, and
  # naming no store path, so a switch never reloads it under its own ask.
  # Every Mac refuses an update during a call, and the follower's switch
  # waits while someone is at the Mac, for 45 minutes at most.
  update-to-main-follows-on-a-mac =
    holds "update-to-main-follows-on-a-mac"
      "A Mac's follower wouldn't start again after follow ends, names a store path a switch would reload it for, or a Mac would switch during a call or under someone at it."
      (
        follows.Label == "org.jitsusama.update-to-main-follows"
        && follows.RunAtLoad
        && !follows.KeepAlive.SuccessfulExit
        && !(lib.hasInfix "/nix/store" (builtins.toJSON follows.ProgramArguments))
        && lib.hasSuffix "/run/current-system/sw/bin/update-to-main follow; exit 75" (
          lib.last follows.ProgramArguments
        )
        && !(darwin.launchd.user.agents ? update-to-main-follows)
        && lib.all (refusal: !refusal.followerWaits) (refusesOnAMac "a call is on")
        && refusesOnAMac "a call is on" != [ ]
        && lib.any (refusal: refusal.followerWaits && refusal.followerWaitsAtMost == 45 * 60) (
          refusesOnAMac "someone is at the machine"
        )
      );
}
