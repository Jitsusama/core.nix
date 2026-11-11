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
  # session-backup sends each settled log once, sealed, keeps and fetches
  # them as its settings say, and indexes what it sends: its tests run as
  # it builds, against a stand-in for Google that refuses any call but a
  # token, a list in the folder, a file made in it and a file read back.
  session-backup-sends-each-log-once = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in session-backup, its tests included.
  session-backup-passes-clippy = passesClippy "session-backup-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # An account that names a folder, or one of the backup's own, gets an
  # hourly run in the background, one that catches up on an hour slept
  # through; one that names none gets nothing.
  session-backup-stays-in-the-background =
    let
      named = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.session-backup ];
            jitsusama.session-backup = {
              folder = "a-folder";
              machine = "a-machine";
              recipients = [ "age1someone" ];
            };
          };
        }
      ]);
      own = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.session-backup ];
            jitsusama.session-backup = {
              ownFolder = "pi sessions";
              machine = "a-machine";
              recipients = [ "age1someone" ];
            };
          };
        }
      ]);
      unnamed = account (bareNixos [
        self.nixosModules.workstation
        { home-manager.users.someone.imports = [ self.homeModules.session-backup ]; }
      ]);
      unit = named.systemd.user.services.session-backup.Unit;
      service = named.systemd.user.services.session-backup.Service;
      timer = named.systemd.user.timers.session-backup.Timer;
    in
    holds "session-backup-stays-in-the-background"
      "session-backup runs outside background.slice, misses an hour slept through, gives up at its first failure, is restarted by a switch, runs for an account that names no folder, or doesn't for one that names only its own."
      (
        service.Slice == "background.slice"
        && service.Restart == "on-failure"
        && unit.X-RestartIfChanged == false
        && timer.Persistent
        && own.systemd.user.services ? session-backup
        && !(unnamed.systemd.user.services ? session-backup)
      );
}
