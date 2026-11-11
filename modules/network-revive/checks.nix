# The checks of network-revive, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # network-revive's tests run as it builds: a look once a burst of changes
  # goes quiet, and each unit's check and whether it reloads read.
  network-revive-looks-once-the-network-settles = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in network-revive, its tests included.
  network-revive-passes-clippy = passesClippy "network-revive-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
