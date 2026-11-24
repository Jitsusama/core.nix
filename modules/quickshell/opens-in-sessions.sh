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
# with neither by then, a kitty that turns the launch down or can't be
# reached, or an old kitty that opens nothing, the journal says why, and
# the shell is told, so it stops counting a viewer (pi-hosted) as coming
# and asks again at the next press (Sessions' untaken); it counts no other
# as coming, so ignores the rest.
# It's told when this was started, so a press made since, whose window
# may yet come, is left coming. The kitty's answer is waited for, so a
# launch it turns down is said, for a minute: kitten's own wait for a
# launch's is a day (kitty's launch --response-timeout, which its Go
# kitten takes as the wait for any command's answer, in tools/cmd/at's
# template.go), and the shell asks again after half a minute anyway.
#
# niri gives what it starts no output (its utils/spawning.rs sets stdin,
# stdout and stderr to null), so a run given none runs again once with
# them in the journal, told when it was first started; one given output
# of its own, as a test's, keeps it.

asked=$(date +%s%3N)
if [ -z "${OPENS_IN_SESSIONS_ASKED:-}" ] && [ /proc/self/fd/2 -ef /dev/null ] &&
  command -v systemd-cat > /dev/null; then
  OPENS_IN_SESSIONS_ASKED=$asked exec systemd-cat --identifier=opens-in-sessions \
    sh "$0" "$@"
fi
asked=${OPENS_IN_SESSIONS_ASKED:-$asked}
unset OPENS_IN_SESSIONS_ASKED
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
  kitten @ --to "unix:$sock" launch --response-timeout 60 --type=os-window \
    --os-window-class "$class" --cwd "$dir" --env "MYCELIUM_APP_ID=$class" "$@" \
    > /dev/null && exit 0
  echo "sessions: the sessions kitty opened no $class" >&2
elif old; then
  systemd-run --user --scope --collect --quiet --slice=agents.slice -- \
    env "MYCELIUM_APP_ID=$class" kitty -1 --instance-group pi-sessions --class "$class" \
    --directory "$dir" "$@" && exit 0
  echo "sessions: the sessions kitty opened no $class" >&2
else
  echo "sessions: no sessions kitty to open $class in" >&2
fi
# Within 5 s, so a shell stalled or starting again leaves no script behind.
timeout 5 qs ipc call sessions untaken "$class" "$asked" > /dev/null 2>&1 ||
  echo "sessions: the shell didn't hear that no kitty took $class" >&2
exit 1
