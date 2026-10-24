# The checks of lid, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in lid-docked, its tests included.
  lid-docked-passes-clippy = passesClippy "lid-docked-passes-clippy" (
    pkgs.callPackage ./docked/package.nix { }
  );
}
