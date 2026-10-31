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
in
{
  # update-to-main's tests run as it builds: a signed main is built and
  # switched to, main where it was builds nothing, an unsigned main, one
  # signed by a key not given and a rewritten one are refused, a failed
  # build leaves the machine as it was, a failed switch and a machine not
  # whole after it go back, a new kernel waits for the next boot, a low
  # battery starts nothing, asks during an update join it, anything but the
  # one ask is refused, and the socket is only its group's to write.
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
  # its askers are its group's members.
  update-to-main-outlives-its-switch =
    holds "update-to-main-outlives-its-switch"
      "A switch could restart update-to-main while it switches, the Mac's daemon doesn't wait for its program, or its askers aren't its group's."
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
      );
}
