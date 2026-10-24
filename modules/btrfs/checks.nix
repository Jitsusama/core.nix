# The checks of btrfs, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  lib,
  onLinux,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # systemd-tmpfiles turns Btrfs's reclaim on for every filesystem mounted,
  # data and metadata, and leaves the system chunks alone. It writes into a
  # copy of sysfs with two filesystems, since a pattern that matches none
  # passes without a word (tests/btrfs-reclaims-its-chunks.sh).
  btrfs-reclaims-its-chunks =
    let
      rules = lib.filter (lib.hasInfix "/sys/fs/btrfs/") workstation.config.systemd.tmpfiles.rules;
    in
    pkgs.runCommandLocal "btrfs-reclaims-its-chunks" { nativeBuildInputs = [ pkgs.systemd ]; } ''
      ${lib.getExe pkgs.bash} ${../../tests/btrfs-reclaims-its-chunks.sh} ${pkgs.writeText "btrfs.conf" (lib.concatLines rules)}
      touch $out
    '';
}
