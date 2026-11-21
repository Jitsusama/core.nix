# The checks of call-under-load, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# call-under-load drives Chrome on Linux, and builds only there.
onLinux {
  # call-under-load's tests run as it builds: the call's frame rates are
  # counted by frame slots, its freezes per ten minutes, the calendar
  # keeps a run ten minutes from a meeting and refuses a stale read, the
  # apps' waits come from lag-watch's last hour, and the hour is written
  # in cadence's shape.
  call-under-load-tells-the-hour = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in call-under-load, its tests included.
  call-under-load-passes-clippy = passesClippy "call-under-load-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
