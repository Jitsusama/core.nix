# shellcheck shell=bash
# Runs a command with the notices daemon (modules/notices) serving on its
# session bus, as the account's unit does on the desktop, for a check that
# runs the shell without systemd (tests/nested-niri.sh). The daemon is
# given in NOTICES_DAEMON as the unit's ExecStart, whose %t and %S, the
# runtime and state folders systemd would put in, are the check's own. A
# notice sent before the daemon holds the bus's name would go nowhere, so
# the command starts once it does, and the daemon ends when the command
# does.
# usage: NOTICES_DAEMON='<the unit's ExecStart>' notices-beside.sh <command>...
set -euo pipefail

# shellcheck disable=SC2154 # NOTICES_DAEMON is the check's, set in its environment.
given=${NOTICES_DAEMON//%t/$XDG_RUNTIME_DIR}
read -ra daemon <<< "${given//%S/$XDG_STATE_HOME}"
mkdir -p "$XDG_RUNTIME_DIR/daemons" "$XDG_STATE_HOME"
"${daemon[@]}" > "${NOTICES_LOG:-/dev/null}" 2>&1 &
served=$!
trap 'kill "$served" 2> /dev/null || true' EXIT

# How long, in tenths of a second, the daemon may take to hold the name.
for _ in $(seq 600); do
  if dbus-send --session --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus \
    org.freedesktop.DBus.NameHasOwner string:org.freedesktop.Notifications 2> /dev/null \
    | grep -q "boolean true"; then
    "$@"
    exit
  fi
  kill -0 "$served" 2> /dev/null || {
    echo "The notices daemon ended before it held the notifications name" >&2
    exit 1
  }
  sleep 0.1
done
echo "The notices daemon didn't hold the notifications name in 60 s" >&2
exit 1
