# How a laptop hibernates: its memory written to the swap file as an image,
# and read back at the next start. Where the image went is kept by systemd in
# an EFI variable, so nothing here names a disk (tests/hibernate.nix proves
# the machine comes back). Coming back asks once, at the disk prompt: the
# lock screen lets go by itself. A hibernation that rolled back, leaving the
# machine on, is tried again or said (unlock/src/main.rs says when).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  unlock = pkgs.callPackage ./unlock/package.nix { };
  disks = lib.attrValues config.boot.initrd.luks.devices;
  # A disk opened with a key file asks nothing, so with one of those no disk
  # is named and the lock always stays.
  asked = lib.optionals (lib.all (disk: disk.keyFile == null) disks) (map (disk: disk.device) disks);
in
{
  # The kernel compresses the image on at most three threads, so the
  # compressor sets how long going down and coming back take. lz4 compresses
  # a little faster than lzo, the kernel's default, and decompresses several
  # times faster, for a slightly larger image. The image's header says which
  # wrote it, so a resume reads it with the same one, whatever the starting
  # kernel was told.
  boot.kernelParams = [ "hibernate.compressor=lz4" ];
  # The image is read in the initrd, so lz4 has to be there before the disk
  # opens. Where a kernel builds it in, as the laptops' does, this asks for
  # nothing more.
  boot.initrd.kernelModules = [ "lz4" ];

  # systemd-sleep runs each program here before a sleep and after it, with
  # "pre" or "post" and the sleep's name.
  environment.etc."systemd/system-sleep/hibernate-unlock".source =
    pkgs.writeShellScript "hibernate-unlock" ''
      PATH=${
        lib.makeBinPath [
          pkgs.cryptsetup
          pkgs.libnotify
          pkgs.systemd
          pkgs.tpm2-tools
        ]
      }
      exec ${lib.getExe unlock} "$1" "$2" ${lib.escapeShellArgs asked}
    '';
}
