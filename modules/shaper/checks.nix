# The checks of shaper, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
{
  # The shaper's tests run as it builds: a queue is read from delayed pings
  # and not from one late one or a host that stays silent, the rate cuts to
  # under what the connection carried and climbs while nothing waits, and
  # each set, route and queue is read from what the kernel prints, a
  # recorded week replays through a controller and is scored, and the
  # rebuilt controller sets each direction from that direction's own delay.
  shaper-finds-the-rate = pkgs.callPackage ./package.nix { };
}
// onLinux {
  # clippy finds nothing in the shaper, its tests included.
  shaper-passes-clippy = passesClippy "shaper-passes-clippy" (pkgs.callPackage ./package.nix { });
}
