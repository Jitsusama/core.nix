# shellcheck shell=bash
# Compiles the shell's files into Quickshell's cache once a switch has linked
# them and before it restarts the shell (home.nix's quickshellStartsWarm), so
# the new shell starts from compiled files and listens sooner (./warm.qml).
#
#   quickshell-starts-warm WARMS
#
# WARMS is a JSON list of the shell's QML files, relative to its folder. The
# folder is where the shell reads its files from, so the cache is filled under
# the names the shell looks them up by. Nothing here fails a switch: without a
# Wayland session to compile against, as before the first login, or with the
# warmer refusing, the shell starts as it would have, compiling as it goes.

warms=$1
folder="${XDG_CONFIG_HOME:-$HOME/.config}/quickshell"
runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
# An activation isn't run from the session, so the display is the one niri
# gave the account's systemd.
display="${WAYLAND_DISPLAY:-}"
if [ -z "$display" ]; then
  display=$(XDG_RUNTIME_DIR="$runtime" systemctl --user show-environment 2> /dev/null \
    | sed -n 's/^WAYLAND_DISPLAY=//p') || display=""
fi
if [ -z "$display" ] || [ ! -f "$folder/warm.qml" ]; then
  exit 0
fi

# Wayland rather than offscreen, since a file naming PanelWindow compiles only
# with a Wayland backend loaded, and every singleton fails with it; the
# warmer makes no window, so niri draws nothing for it.
if ! env -u QT_QPA_PLATFORMTHEME \
  XDG_RUNTIME_DIR="$runtime" WAYLAND_DISPLAY="$display" QT_QPA_PLATFORM=wayland \
  WARM_FILES="$(cat "$warms")" \
  timeout 60 quickshell --no-detailed-logs -p "$folder/warm.qml" > /dev/null 2>&1; then
  echo "quickshell-starts-warm: nothing compiled ahead; the shell compiles as it starts" >&2
fi
