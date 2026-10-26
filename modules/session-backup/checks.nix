# The checks of session-backup, which checks.nix gathers with every other
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
  # clippy finds nothing in session-backup, its tests included, which run
  # as it builds against a stand-in for Google that refuses any call but a
  # token, a list in the folder and a file made in it.
  session-backup-passes-clippy = passesClippy "session-backup-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # An account that names a folder gets an hourly run in the background,
  # one that catches up on an hour slept through; one that names none gets
  # nothing.
  session-backup-stays-in-the-background =
    let
      named = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.session-backup ];
            jitsusama.session-backup = {
              folder = "a-folder";
              host = "a-machine";
              recipients = [ "age1someone" ];
            };
          };
        }
      ]);
      unnamed = account (bareNixos [
        self.nixosModules.workstation
        { home-manager.users.someone.imports = [ self.homeModules.session-backup ]; }
      ]);
      service = named.systemd.user.services.session-backup.Service;
      timer = named.systemd.user.timers.session-backup.Timer;
    in
    holds "session-backup-stays-in-the-background"
      "session-backup runs outside background.slice, misses an hour slept through, or runs for an account that names no folder."
      (
        service.Slice == "background.slice"
        && timer.Persistent
        && !(unnamed.systemd.user.services ? session-backup)
      );
}
