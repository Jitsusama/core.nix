# The checks of ir-camera, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # ir-camera's tests run as it builds: each sample's highest eight bits
  # kept, lines their stride apart, and a short frame refused.
  ir-camera-relays-grey-frames = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in ir-camera, its tests included.
  ir-camera-passes-clippy = passesClippy "ir-camera-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
