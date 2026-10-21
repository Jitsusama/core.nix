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
    text = ''
      marker=$XDG_RUNTIME_DIR/quickshell-locked
      drawn() { [ "$(cat "$marker" 2> /dev/null)" = locked ]; }

      # Watched before asking, so the lock can't be drawn unseen in between.
      exec 3< <(exec inotifywait -m -e moved_to,close_write --format %f "$XDG_RUNTIME_DIR" 2>&1)
      watcher=$!
      trap 'kill "$watcher" 2> /dev/null || true' EXIT
      while read -r line <&3 && [ "$line" != "Watches established." ]; do :; done

      qs ipc call lock lock
      deadline=$((SECONDS + 3))
      until drawn; do
        left=$((deadline - SECONDS))
        if [ "$left" -le 0 ] || ! read -r -t "$left" _ <&3; then
          drawn && exit 0
          echo "the lock screen wasn't drawn in time" >&2
          exit 1
        fi
      done
    '';
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
