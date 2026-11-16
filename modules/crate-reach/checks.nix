# The checks of crate-reach, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# merge-net and the gates' replay read it on Linux only, and a Mac builds
# it as part of nix-offload, whose own build runs its code there.
onLinux {
  # Its tests run as it builds: what main moved in a crate a change's
  # crate reads, or one reading it, however far, is beside the change,
  # and crates that never read each other stay apart.
  crate-reach-finds-what-reads-what = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in crate-reach, its tests included.
  crate-reach-passes-clippy = passesClippy "crate-reach-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
