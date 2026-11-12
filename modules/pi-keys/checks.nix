# The checks of pi-keys, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# pi-keys reads Linux's per-thread counts, and builds only there.
onLinux {
  # pi-keys' tests run as it builds: a thread's bursts from its counts, a
  # key's waits modelled from them, and the window's threads found as the
  # jobs-sorter finds them.
  pi-keys-models-a-key-s-waits = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in pi-keys, its tests included.
  pi-keys-passes-clippy = passesClippy "pi-keys-passes-clippy" (pkgs.callPackage ./package.nix { });
}
