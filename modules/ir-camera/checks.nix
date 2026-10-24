# The checks of ir-camera, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in ir-camera, its tests included.
  ir-camera-passes-clippy = passesClippy "ir-camera-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
