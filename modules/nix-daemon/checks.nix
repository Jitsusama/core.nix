# The checks of nix-daemon, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
{
  # Nix's builds yield to the user's programs only if their slice sits beside
  # user.slice, at a lower weight. A dash in its name nests it in a parent at
  # the default weight, and nothing says so.
  nix-builds-yield-to-the-user =
    let
      inherit (workstation.config) systemd;
      slice = systemd.services.nix-daemon.serviceConfig.Slice;
      name = lib.removeSuffix ".slice" slice;
    in
    holds "nix-builds-yield-to-the-user"
      "The Nix daemon's slice, ${slice}, isn't beside user.slice at a lower weight."
      (
        !(lib.hasInfix "-" name)
        &&
          (systemd.slices.${name}.sliceConfig.CPUWeight or 100)
          < (systemd.slices.user.sliceConfig.CPUWeight or 100)
      );

  # vm-first's tests run as it builds: it reads a thread's nice and policy
  # past any name, knows qemu wrapped or not and nothing else, and counts a
  # thread behind until it is both normal and at the nice asked for.
  vm-first-finds-the-vms = pkgs.callPackage ./vm-first/package.nix { };

  # Nix collects garbage during a build once free space falls under min-free,
  # and stops at max-free. A max-free at or under min-free stops it before it
  # starts, and nothing says so.
  nix-collects-before-the-disk-fills =
    let
      inherit (workstation.config.nix.settings) min-free max-free;
    in
    holds "nix-collects-before-the-disk-fills"
      "The Nix daemon never collects garbage during a build: min-free is ${toString min-free}, max-free ${toString max-free}."
      (min-free > 0 && max-free > min-free);
}
// onLinux {
  # clippy finds nothing in vm-first, its tests included.
  vm-first-passes-clippy = passesClippy "vm-first-passes-clippy" (
    pkgs.callPackage ./vm-first/package.nix { }
  );

  # without-syncfs's tests run as it builds: the kernel answers a program it
  # runs ENOSYS for syncfs and runs its fsync, under one more seccomp filter
  # than without it, since a build here is refused syncfs by the daemon too.
  without-syncfs-refuses-only-syncfs = pkgs.callPackage ./without-syncfs/package.nix { };

  # clippy finds nothing in without-syncfs, its tests included.
  without-syncfs-passes-clippy = passesClippy "without-syncfs-passes-clippy" (
    pkgs.callPackage ./without-syncfs/package.nix { }
  );
}
