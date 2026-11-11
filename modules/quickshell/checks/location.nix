# The checks of where the machine is: the clock that follows its time zone, and
# whereabouts, which the folder's checks.nix gathers with every other place's;
# its opening comment says why each place keeps its own.
{
  passesClippy,
  pkgs,
  ...
}:
{
  # clippy finds nothing in clock-follows, its tests included.
  clock-follows-passes-clippy = passesClippy "clock-follows-passes-clippy" (
    pkgs.callPackage ../clock-follows/package.nix { }
  );

  # clippy finds nothing in whereabouts, its tests included.
  whereabouts-passes-clippy = passesClippy "whereabouts-passes-clippy" (
    pkgs.callPackage ../whereabouts/package.nix { }
  );
}
