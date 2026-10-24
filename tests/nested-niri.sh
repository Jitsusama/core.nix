# shellcheck shell=bash
# Runs a command against a real niri, nested in a headless sway drawn in
# software, inside the build sandbox: the desktop's compositor without a
# machine to boot. The command finds niri through WAYLAND_DISPLAY and
# NIRI_SOCKET, as the account's programs do, and its session bus through
# DBUS_SESSION_BUS_ADDRESS. No keyboard types into niri here: wtype's keys,
# on either display, never reached niri's binds, so a check runs what a
# bind runs rather than pressing it. Each compositor's log goes in the
# folder given, which is printed when the command fails, and both end when
# it does; what the command starts, it ends itself.
# usage: nested-niri.sh <niri config> <logs> <command>...
set -euo pipefail

config=$1
logs=$2
shift 2
mkdir -p "$logs"
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-$(mktemp -d)}
chmod 700 "$XDG_RUNTIME_DIR"
# How long, in tenths of a second, each compositor may take to open its
# socket.
starting=600

# Waits for a path to exist, or says what didn't start and fails.
appears() {
  local what=$1 path tries
  for tries in $(seq "$starting"); do
    for path in "${@:2}"; do
      [ -S "$path" ] && return 0
    done
    sleep 0.1
  done
  echo "$what didn't start in $((tries / 10)) s" >&2
  tail -n 40 "$logs"/*.log >&2
  exit 1
}

# Ends both compositors when this ends. Not its whole process group, which
# a build shares with the builder itself.
# shellcheck disable=SC2046 # Each job's id is a word of its own.
trap 'kill $(jobs -p) 2> /dev/null || true' EXIT

printf 'output HEADLESS-1 resolution 1280x800\n' > "$logs/sway.conf"
WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 \
  sway -c "$logs/sway.conf" > "$logs/sway.log" 2>&1 &
appears sway "$XDG_RUNTIME_DIR/wayland-1"

WAYLAND_DISPLAY=wayland-1 niri -c "$config" > "$logs/niri.log" 2>&1 &
appears niri "$XDG_RUNTIME_DIR/wayland-2"
for _ in $(seq "$starting"); do
  sockets=("$XDG_RUNTIME_DIR"/niri.*.sock)
  [ -S "${sockets[0]}" ] && break
  sleep 0.1
done
export WAYLAND_DISPLAY=wayland-2 NIRI_SOCKET=${sockets[0]}

# The session bus's own configuration, as dbus ships it, since a build has
# no /etc.
bus=$(dirname "$(command -v dbus-daemon)")/../share/dbus-1/session.conf
status=0
dbus-run-session --config-file="$bus" -- "$@" || status=$?
if [ "$status" != 0 ]; then
  tail -n 40 "$logs"/*.log >&2
fi
exit "$status"
