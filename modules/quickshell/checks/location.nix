# The checks of where the machine is: the clock that follows its time zone, and
# whereabouts, which the folder's checks.nix gathers with every other place's;
# its opening comment says why each place keeps its own.
{
  account,
  graphical,
  holds,
  lib,
  passesClippy,
  pkgs,
  ...
}:
{
  # Typed bare in a terminal, clock-follows says where the clock last
  # moved, so wherever its service runs it's on the account's path too.
  # commands-agree runs the bare command against a stand-in of its state.
  clock-follows-is-on-the-path =
    let
      home = account graphical;
      carried = lib.any (package: lib.getName package == "clock-follows") home.home.packages;
    in
    holds "clock-follows-is-on-the-path"
      "clock-follows should be on the path of every account whose clock follows the laptop"
      (home.systemd.user.services ? clock-follows && carried);

  # clippy finds nothing in clock-follows, its tests included.
  clock-follows-passes-clippy = passesClippy "clock-follows-passes-clippy" (
    pkgs.callPackage ../clock-follows/package.nix { }
  );

  # clippy finds nothing in whereabouts, its tests included.
  whereabouts-passes-clippy = passesClippy "whereabouts-passes-clippy" (
    pkgs.callPackage ../whereabouts/package.nix { }
  );
}
