# How a laptop hibernates: its memory written to the swap file as an image,
# and read back at the next start. Where the image went is kept by systemd in
# an EFI variable, so nothing here names a disk (tests/hibernate.nix proves
# the machine comes back). Coming back asks once, at the disk prompt: the
# lock screen lets go by itself (unlock/src/main.rs says when).
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
  # Compressing the image is most of how long going down and coming back
  # take. lz4 compresses a little faster than lzo, the kernel's default, and
  # decompresses several times faster, for a slightly larger image. The
  # image's header says which wrote it, so a resume reads it with the same
  # one, whatever the starting kernel was told.
  #
  # The kernel compresses and decompresses on at most
  # hibernate_compression_threads threads, three unless told, and never more
  # than every CPU but one; three wrote 22 GB in 41 s. Asking for 64 gives
  # every CPU but one, whatever the laptop, and each thread needs only two
  # small buffers. On the command line it reaches the starting kernel too,
  # which reads the image back.
  boot.kernelParams = [
    "hibernate.compressor=lz4"
    "hibernate_compression_threads=64"
  ];
  # The same, written to the running kernel, so a switch needs no restart.
  # A kernel without the file is left alone.
  systemd.tmpfiles.settings.hibernate."/sys/power/hibernate_compression_threads".w.argument = "64";
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
          pkgs.systemd
          pkgs.tpm2-tools
        ]
      }
      exec ${lib.getExe unlock} "$1" "$2" ${lib.escapeShellArgs asked}
    '';
}
