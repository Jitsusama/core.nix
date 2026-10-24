# The checks of lag-watch, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
{
  # lag-watch's tests run as it builds: the desktop or apps waiting is
  # trouble and work waiting on work isn't, niri slow to answer is, the
  # watched threads are found by name, and the report sums each
  # generation by its load.
  lag-watch-tells-who-waited = pkgs.callPackage ./package.nix { };
}
// onLinux {
  # clippy finds nothing in lag-watch, its tests included.
  lag-watch-passes-clippy = passesClippy "lag-watch-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
