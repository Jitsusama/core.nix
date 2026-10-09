# swayidle locks the screen when Joel steps away, before the laptop sleeps and
# whenever logind asks (loginctl lock-session), through Quickshell's lock
# screen. A while after locking, niri turns the screens off; any key or touch
# turns them back on.
{ lib, pkgs, ... }:
let
  lock = "${lib.getExe' pkgs.quickshell "qs"} ipc call lock lock";
in
{
  services.swayidle = {
    enable = true;
    events = {
      before-sleep = lock;
      inherit lock;
    };
    timeouts = [
      {
        timeout = 300;
        command = lock;
      }
      {
        timeout = 600;
        command = "${lib.getExe pkgs.niri} msg action power-off-monitors";
      }
    ];
  };
}
