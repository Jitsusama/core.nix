# The checks of network-revive, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in network-revive, its tests included.
  network-revive-passes-clippy = passesClippy "network-revive-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
