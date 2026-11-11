# How a laptop hibernates: its memory written to the swap file as an image,
# and read back at the next start. Where the image went is kept by systemd in
# an EFI variable, so nothing here names a disk (tests/vm/disks/hibernate.nix
# proves the machine comes back). Coming back asks once, at the disk prompt: the
# lock screen lets go by itself. A hibernation that rolled back, leaving the
# machine on, is said with why, tried again or slept through instead
# (unlock/src/main.rs says when). While the image is made the slices' hard
# memory floors are set aside, or the kernel can't take back what they guard
# to make room (unlock/src/floors.rs), zswap is switched off, or what's
# swapped out for it can stay in memory (unlock/src/zswap.rs), and
# `hibernate-unlock fits` says
# beforehand whether the image would fit, by the kernel's own sums. Once a
# restore is back, the hook says how long it took (unlock/src/restored.rs).
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
  inherit (config.jitsusama.hibernate) reserve;
in
{
  options.jitsusama.hibernate.reserve = lib.mkOption {
    type = lib.types.nullOr lib.types.ints.positive;
    default = null;
    example = 1024 * 1024 * 1024;
    description = ''
      The memory, in bytes, the kernel leaves free for what drivers allocate
      once the image is sized, as they stop their devices; null leaves the
      kernel's own 1 MB. The image is held to half of memory less twice
      this, and as much again is set aside to copy it into, so about four
      times this stays free. A laptop whose graphics driver copies its own
      memory out as it stops sets one to cover that, or the copy runs short
      and the hibernation rolls back.
    '';
  };

  config = {
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
    # Each written to the running kernel at every start and switch. A kernel
    # without the file is left alone.
    systemd.tmpfiles.settings.hibernate = {
      # The same, so a switch needs no restart.
      "/sys/power/hibernate_compression_threads".w.argument = "64";
      # The kernel has no parameter for it on its command line.
      "/sys/power/reserved_size" = lib.mkIf (reserve != null) { w.argument = toString reserve; };
    };
    # The image is read in the initrd, so lz4 has to be there before the disk
    # opens. Where a kernel builds it in, as the laptops' does, this asks for
    # nothing more.
    boot.initrd.kernelModules = [ "lz4" ];

    # Just before the image is read, the initrd keeps when, in an EFI
    # variable, so the hook can say how long the restore took once the
    # machine is back: the image's kernel takes the initrd's memory and its
    # log with it (unlock/src/restored.rs). systemd's generator starts this
    # unit only when there's an image to look for, and nothing kept stands
    # in the restore's way.
    boot.initrd.systemd.services.systemd-hibernate-resume.serviceConfig.ExecStartPre = [
      "-${lib.getExe unlock} restoring"
    ];
    boot.initrd.systemd.storePaths = [ (lib.getExe unlock) ];

    # For `hibernate-unlock fits`, for whatever asks for a hibernation to run
    # first: asked inside the sleep, it would come after the screen locked.
    # Nothing here asks it.
    environment.systemPackages = [ unlock ];

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
  };
}
