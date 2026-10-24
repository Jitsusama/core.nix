# The checks of backlight, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in the backlight daemon, its tests included.
  backlight-passes-clippy = passesClippy "backlight-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
