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
  # Nix's builds yield to the person's programs only if their slice sits beside
  # user.slice, at a lower weight. A dash in its name nests it in a parent at
  # the default weight, and nothing says so.
  nix-builds-yield-to-the-person =
    let
      inherit (workstation.config) systemd;
      slice = systemd.services.nix-daemon.serviceConfig.Slice;
      name = lib.removeSuffix ".slice" slice;
    in
    holds "nix-builds-yield-to-the-person"
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
  # starts, and nothing says so. It's the last resort beneath the disk guard,
  # so it starts only past the guard's urgent band, a tenth free
  # (modules/disk-guard/src/config.rs), on a disk as small as 120 GB: above
  # it, every add to a store the guard had cleared began a collection that
  # couldn't free what it wanted, holding the store's lock inside the add.
  nix-collects-before-the-disk-fills =
    let
      inherit (workstation.config.nix.settings) min-free max-free;
      urgent = 120 * 1000 * 1000 * 1000 / 10;
    in
    holds "nix-collects-before-the-disk-fills"
      "The Nix daemon never collects garbage during a build, or does before the disk guard: min-free is ${toString min-free}, max-free ${toString max-free}."
      (min-free > 0 && max-free > min-free && min-free < urgent);
}
// onLinux {
  # clippy finds nothing in vm-first, its tests included.
  vm-first-passes-clippy = passesClippy "vm-first-passes-clippy" (
    pkgs.callPackage ./vm-first/package.nix { }
  );

  # without-syncfs's tests run as it builds: the kernel answers a program it
  # runs ENOSYS for syncfs and runs its fsync, under one more seccomp filter
  # than without it, since the daemon answers a build's syncfs as done too.
  without-syncfs-refuses-only-syncfs = pkgs.callPackage ./without-syncfs/package.nix { };

  # The door sandboxes are given in place of the daemon's socket answers as
  # Nix's daemon and takes its caller as untrusted, run as its unit runs it.
  # A Nix that renames a flag or a feature it needs leaves the door shut, or
  # worse, open to a trusted caller, and nothing else would say so.
  the-sandboxes-door-trusts-no-one =
    let
      inherit (workstation.config.systemd.user) services;
    in
    pkgs.runCommandLocal "the-sandboxes-door-trusts-no-one" { } ''
      ${lib.getExe pkgs.bash} ${./tests/the-sandboxes-door-trusts-no-one.sh} \
        ${lib.getExe' pkgs.systemd "systemd-socket-activate"} \
        ${lib.getExe' workstation.config.nix.package "nix"} \
        ${services."nix-untrusted@".serviceConfig.ExecStart}
      touch $out
    '';

  # clippy finds nothing in without-syncfs, its tests included.
  without-syncfs-passes-clippy = passesClippy "without-syncfs-passes-clippy" (
    pkgs.callPackage ./without-syncfs/package.nix { }
  );
}
