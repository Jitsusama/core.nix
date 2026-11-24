# The checks of call-under-load, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# call-under-load drives Chrome on Linux, and builds only there.
onLinux {
  # A switch leaves the hourly run and the camera check be, so it never
  # waits on a run under way, which failed switch 1186 on 24 November.
  call-under-load-outlives-a-switch =
    let
      services = (account graphical).systemd.user.services;
      kept = name: (services.${name}.Unit.X-SwitchMethod or null) == "keep-old";
    in
    holds "call-under-load-outlives-a-switch"
      "A switch restarts call-under-load's hourly run or camera check, and waits on it."
      (kept "call-under-load" && kept "call-under-load-camera");

  # call-under-load's tests run as it builds: the call's frame rates are
  # counted by frame slots, its freezes per ten minutes, whether its
  # sound was heard, the share made up, its jitter and packets lost, and
  # why any is missing, and that only Chromium's fake devices are taken, the calendar
  # keeps a run ten minutes from a meeting and refuses a stale read, the
  # apps' waits come from lag-watch's last hour, the launcher's openings
  # during the run are set against the span before on their medians, or
  # can't be told under three a side or from a journal not read whole,
  # the hour is written in cadence's shape, and Joel's own calls' sending
  # waits are summed into a line the hour reads and no test run is taken for,
  # and the camera is checked once, 18 to 22 minutes before each meeting and
  # never during one or the quarter hour before,
  # one plugged in he can open taken over the machine's, only a loopback
  # read and its frames sent to /dev/null, the notice's words plain, and a
  # program given its time and no more.
  call-under-load-tells-the-hour = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in call-under-load, its tests included.
  call-under-load-passes-clippy = passesClippy "call-under-load-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
