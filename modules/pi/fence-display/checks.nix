# The checks of pi-fence-display, the sandboxed display pi's fenced
# commands open windows on; what it offers on niri is checked beside the
# clipboard door's (../clipboard-door/checks.nix).
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in pi-fence-display, its tests included.
  pi-fence-display-passes-clippy = passesClippy "pi-fence-display-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
