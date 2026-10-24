# swayidle locks the screen when Joel steps away, before the laptop sleeps and
# whenever logind asks (loginctl lock-session), through Quickshell's lock
# screen. A while after locking, niri turns the screens off; any key or touch
# turns them back on.
#
# Asking for the lock returns before it's drawn, and niri shows the desktop
# until every screen has its lock, so a machine let sleep at once could wake
# showing the desktop. The lock command returns once Quickshell says niri has
# drawn it (Lock.qml), and swayidle holds the sleep until it does, for up to
# three seconds, well inside logind's five.
#
# The screens go off through the system's niri, the one with core's patches,
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
in
{
  home.packages = [ lock ];

  services.swayidle = {
    enable = true;
    events = {
      before-sleep = lib.getExe lock;
      lock = lib.getExe lock;
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
