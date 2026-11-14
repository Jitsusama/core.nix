#!/bin/sh
# Opens one session's window in the sessions kitty, as Sessions.qml's
# inSessions asks it, decided as the window opens, in what niri starts,
# so the shell waits on nothing.
#
# usage: opens-in-sessions.sh SOCKET CLASS [--directory DIR] COMMAND...
#
# A window is asked of that kitty over its remote-control socket, in a
# runtime folder only Joel's login opens (sessions-kitty's unit), never
# through kitty's single instance, whose name any program could take
# first and so be handed every session's window (ISXEG8, 2026-11-11). The
# window gets kitty's own environment and only its app id beside it.
#
# A kitty started before its unit listened has no socket until the next
# login, so until then it's reached the old way, but only while it is the
# unit's own main process, which nothing else can be. A kitty its unit is
# starting again, 5 s after it ended, is waited on for up to 20 s, so a
# window asked of it meanwhile opens once it's back rather than never;
# with neither by then, nothing opens, and the journal says why.

sock=$1
class=$2
shift 2
dir=$HOME
if [ "$1" = --directory ]; then
  dir=$2
  shift 2
fi

# Whether the unit's main process is a kitty from before it listened.
old() {
  pid=$(systemctl --user show --property=MainPID --value sessions-kitty.service)
  [ "${pid:-0}" -gt 0 ] &&
    tr "\0" " " < "/proc/$pid/cmdline" | grep -q -- "--instance-group pi-sessions"
}

tries=100
until [ -S "$sock" ] || old; do
  tries=$((tries - 1))
  [ "$tries" -gt 0 ] || break
  sleep 0.2
done

if [ -S "$sock" ]; then
  exec kitten @ --to "unix:$sock" launch --type=os-window --os-window-class "$class" \
    --cwd "$dir" --env "MYCELIUM_APP_ID=$class" "$@"
fi
if old; then
  exec systemd-run --user --scope --collect --quiet --slice=agents.slice -- \
    env "MYCELIUM_APP_ID=$class" kitty -1 --instance-group pi-sessions --class "$class" \
    --directory "$dir" "$@"
fi
echo "sessions: no sessions kitty to open $class in" >&2
exit 1
