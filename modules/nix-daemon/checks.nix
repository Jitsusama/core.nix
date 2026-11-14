# The checks of nix-daemon, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareDarwin,
  holds,
  lib,
  macWorkstation,
  onLinux,
  passesClippy,
  pkgs,
  self,
  workstation,
  ...
}:
{
  # A Mac's daemon signs only if the line naming its key is written for its
  # Nix to include, and the key is made before /etc is, under the name it
  # was given or, with none, the Mac's own. Nothing else would say a Mac
  # signed nothing until another machine refused what it built.
  a-mac-signs-what-it-builds =
    let
      inherit (macWorkstation) config;
      made = config.system.activationScripts.preActivation.text;
      named =
        (bareDarwin [
          self.darwinModules.nix-daemon
          { jitsusama.nix-daemon.keyName = "mac-1"; }
        ]).config.system.activationScripts.preActivation.text;
    in
    holds "a-mac-signs-what-it-builds"
      "A Mac's Nix isn't told to sign with /etc/nix/signing-key, or the key isn't made under its name."
      (
        config.environment.etc."nix/signing.conf".text == "secret-key-files = /etc/nix/signing-key\n"
        && lib.hasInfix "-signing-key.sh " made
        && lib.hasInfix "scutil --get LocalHostName)-1" made
        && lib.hasInfix " mac-1 /etc/nix/signing-key /etc/nix/signing-key.pub" named
      );

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

  # The door's daemon, run as its socket unit runs it, hands a client that
  # isn't trusted only the impure-env variables its nix.conf names, and only to
  # a fixed-output build, however the client asks, and puts no value in a
  # warning or a log. Stock Nix 2.34 hands it none, so a tool that fetches
  # private sources fails from an untrusted account or a sandbox; a patch
  # that let any name through would let a client set what the daemon set
  # for everyone's downloads. The door runs the machine's own Nix, so this
  # proves the daemon's too.
  an-untrusted-client-passes-only-named-env =
    let
      inherit (workstation.config.systemd.user) services;
    in
    pkgs.runCommandLocal "an-untrusted-client-passes-only-named-env" { } ''
      ${lib.getExe pkgs.bash} ${./tests/an-untrusted-client-passes-only-named-env.sh} \
        ${lib.getExe' pkgs.systemd "systemd-socket-activate"} \
        ${lib.getExe' workstation.config.nix.package "nix"} \
        ${./tests/impure-env-build.nix} \
        ${services."nix-untrusted@".serviceConfig.ExecStart}
      touch $out
    '';

  # clippy finds nothing in without-syncfs, its tests included.
  without-syncfs-passes-clippy = passesClippy "without-syncfs-passes-clippy" (
    pkgs.callPackage ./without-syncfs/package.nix { }
  );
}
