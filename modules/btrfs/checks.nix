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
  # passes without a word.
  btrfs-reclaims-its-chunks =
    let
      rules = lib.filter (lib.hasInfix "/sys/fs/btrfs/") workstation.config.systemd.tmpfiles.rules;
    in
    pkgs.runCommandLocal "btrfs-reclaims-its-chunks" { nativeBuildInputs = [ pkgs.systemd ]; } ''
      for fs in one two; do
        for kind in data metadata system; do
          mkdir -p root/sys/fs/btrfs/$fs/allocation/$kind
          for setting in dynamic_reclaim periodic_reclaim; do
            echo 0 > root/sys/fs/btrfs/$fs/allocation/$kind/$setting
          done
        done
      done
      systemd-tmpfiles --root=$PWD/root --create ${pkgs.writeText "btrfs.conf" (lib.concatLines rules)}
      for fs in one two; do
        for setting in dynamic_reclaim periodic_reclaim; do
          for kind in data metadata; do
            if [ "$(cat root/sys/fs/btrfs/$fs/allocation/$kind/$setting)" != 1 ]; then
              echo "Btrfs's $setting is off for $kind."
              exit 1
            fi
          done
          if [ "$(cat root/sys/fs/btrfs/$fs/allocation/system/$setting)" != 0 ]; then
            echo "Btrfs's $setting was turned on for the system chunks."
            exit 1
          fi
        done
      done
      touch $out
    '';
}
