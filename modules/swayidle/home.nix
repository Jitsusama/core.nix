# swayidle locks the screen when Joel steps away, before the laptop sleeps and
# whenever logind asks (loginctl lock-session), through Quickshell's lock
# screen. A while after locking, niri turns the screens off; any key or touch
# turns them back on.
#
# Asking for the lock returns before it's drawn, and niri shows the desktop
# until every screen has its lock, so a machine let sleep at once could wake
# showing the desktop. The lock command returns once Quickshell says niri has
# drawn it (Lock.qml), and swayidle holds the sleep until it does, for up to
# three seconds, well inside logind's five. Then the screens go off
# (lock-for-sleep.sh), so a hibernation goes down dark, and they come back
# on once the machine has woken, to the lock after a suspend.
#
# When logind says to unlock, as after a resume from hibernation, where the
# disk's prompt has already asked for the password, the lock lets go without
# asking again.
#
# The screens go off and on through the system's niri, the one with core's patches,
# so the machine doesn't carry a second niri just to ask.
{
  lib,
  pkgs,
  osConfig ? { },
  ...
}:
let
  niri = osConfig.programs.niri.package or pkgs.niri;
  lock = pkgs.writeShellApplication {
    name = "lock-screen";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.inotify-tools
      pkgs.quickshell
    ];
    text = builtins.readFile ./lock-screen.sh;
  };
  lockForSleep = pkgs.writeShellApplication {
    name = "lock-for-sleep";
    runtimeInputs = [
      lock
      niri
    ];
    text = builtins.readFile ./lock-for-sleep.sh;
  };
  screensOn = "${lib.getExe niri} msg action power-on-monitors";
in
{
  home.packages = [
    lock
    lockForSleep
  ];

  services.swayidle = {
    enable = true;
    events = {
      before-sleep = lib.getExe lockForSleep;
      after-resume = screensOn;
      lock = lib.getExe lock;
      unlock = "${lib.getExe' pkgs.quickshell "qs"} ipc call lock release";
    };
    timeouts = [
      {
        timeout = 300;
        command = lib.getExe lock;
      }
      {
        timeout = 600;
        command = "${lib.getExe niri} msg action power-off-monitors";
      }
    ];
  };
}
