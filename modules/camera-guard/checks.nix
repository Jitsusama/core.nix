# The checks of camera-guard, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # camera-guard's tests run as it builds: the guard is up from a quarter
  # hour before a meeting until it ends, by call-under-load's reading of a
  # meeting, through a calendar that can't be read and across restarts; a
  # slice made again is guarded at the next look; and its program refuses a
  # camera's node and no other.
  camera-guard-keeps-agents-off-the-camera = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in camera-guard, its tests included.
  camera-guard-passes-clippy = passesClippy "camera-guard-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
