# The checks of lid, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # lid-docked's tests run as it builds: a monitor counted on any connector
  # not disconnected, and the lid held only while one is there.
  lid-docked-holds-the-lid-only-with-a-screen = pkgs.callPackage ./docked/package.nix { };

  # clippy finds nothing in lid-docked, its tests included.
  lid-docked-passes-clippy = passesClippy "lid-docked-passes-clippy" (
    pkgs.callPackage ./docked/package.nix { }
  );
}
