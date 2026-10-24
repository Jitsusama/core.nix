# The checks of systemd, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
{
  # shutdown-status's tests run as it builds: it reads only systemd-shutdown's
  # and the ramfs's steps, first lines only, says forced once for a last stage
  # begun unmarked, and leftovers for three seconds' silence after a wait.
  shutdown-status-reads-the-last-stage = pkgs.callPackage ./shutdown-status/package.nix { };

  # The services that end a healthy stop leave shutdown-status its mark, but a
  # mark that couldn't be written must never stop the reboot, so each ignores
  # its failure. shutdown-status itself has to outlive the last stage's kills,
  # or it can't say what's left, and systemd reads that setting from [Unit]
  # alone, ignoring it anywhere else.
  the-shutdown-mark-cannot-stop-a-reboot =
    let
      machine = bareNixos [
        self.nixosModules.systemd
        { boot.plymouth.enable = true; }
      ];
      inherit (machine.config.systemd) services;
      ends = [
        "reboot"
        "poweroff"
        "halt"
        "kexec"
      ];
      marks = end: services."systemd-${end}".serviceConfig.ExecStartPre;
    in
    holds "the-shutdown-mark-cannot-stop-a-reboot"
      "shutdown-status dies at the last stage, or a shutdown fails when it can't leave a mark."
      (
        services.shutdown-status.unitConfig.SurviveFinalKillSignal or false
        && lib.all (end: lib.hasPrefix "-" (marks end)) ends
      );
}
// onLinux {
  # clippy finds nothing in shutdown-status, its tests included.
  shutdown-status-passes-clippy = passesClippy "shutdown-status-passes-clippy" (
    pkgs.callPackage ./shutdown-status/package.nix { }
  );
}
