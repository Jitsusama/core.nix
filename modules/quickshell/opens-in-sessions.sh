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
# starting again, 5 s after it ended, is waited on for up to 20 s from
# this start, its last look included, which the shell's answerPatience
# counts on. It's timed by the time since boot, which no time sync steps,
# rather than by counting looks, since each look asks systemd and can be
# slow under a cold boot's load; so each look is cut at 2 s, said if it
# is, the first always made and another only while it would begin before
# 17 s, with no pause after the last, so the wait ends by 19 s and its
# programs' starts, the kitty's command line read uncut among them. The
# old kitty's own launch, through systemd-run, isn't cut either, as a cut
# could end the kitty it starts were the old one gone since; it's reached
# only until the next login.
# So a window asked of it meanwhile opens once it's back rather than never;
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
# That minute running out is said as the kitty not answering, never as
# taking no window, since the window may yet come (kitten says so in
# words its at/main.go gives an async command's, such as a launch).
#
# niri gives what it starts no output (its utils/spawning.rs sets stdin,
# stdout and stderr to null), so a run given none runs again once with
# them in the journal, told when it was first started; one given output
# of its own, as a test's, keeps it. Only once the journal is seen to
# take a stream: systemd-cat that can't make one ends before running
# what it was given (its cat.c), which would open no window and tell
# the shell nothing.

# The time since boot in hundredths of a second, from /proc/uptime,
# whose first field always has two decimals, read by cut, as the doors
# test's stand-in for it takes it.
up() {
  since=$(cut -d ' ' -f 1 /proc/uptime)
  echo $((${since%.*} * 100 + 1${since#*.} - 100))
}

# Whether a time is a number as this script writes one: digits with no
# leading zero, few enough for the shell's arithmetic.
number() {
  case $1 in
    '' | 0* | *[!0-9]* | ?????????????????*) return 1 ;;
    *) return 0 ;;
  esac
}

asked=$(date +%s%3N)
started=$(up)
# A first start's times are taken only as its run under the journal is
# given them: with output, both numbers, the start no later than this
# one's and within 5 s of it, as that run follows its first within
# moments. So any other that set them is never evaluated, neither holds
# the wait past its 17 s nor cuts it short, and leaves a run given no
# output still moving to the journal.
if ! [ /proc/self/fd/2 -ef /dev/null ] && number "${OPENS_IN_SESSIONS_ASKED:-}" &&
  number "${OPENS_IN_SESSIONS_STARTED:-}" &&
  [ $((started - OPENS_IN_SESSIONS_STARTED)) -ge 0 ] &&
  [ $((started - OPENS_IN_SESSIONS_STARTED)) -le 500 ]; then
  asked=$OPENS_IN_SESSIONS_ASKED
  started=$OPENS_IN_SESSIONS_STARTED
elif [ /proc/self/fd/2 -ef /dev/null ] && command -v systemd-cat > /dev/null &&
  systemd-cat --identifier=opens-in-sessions true 2> /dev/null; then
  exec env "OPENS_IN_SESSIONS_ASKED=$asked" "OPENS_IN_SESSIONS_STARTED=$started" \
    systemd-cat --identifier=opens-in-sessions sh "$0" "$@"
fi
unset OPENS_IN_SESSIONS_ASKED OPENS_IN_SESSIONS_STARTED
sock=$1
class=$2
shift 2
dir=$HOME
if [ "$1" = --directory ]; then
  dir=$2
  shift 2
fi

# Whether the unit's main process is a kitty from before it listened,
# marking a look systemd didn't answer in time (cut).
old() {
  pid=$(timeout 2 systemctl --user show --property=MainPID --value sessions-kitty.service) ||
    [ $? -ne 124 ] || cut=yes
  [ "${pid:-0}" -gt 0 ] &&
    tr "\0" " " < "/proc/$pid/cmdline" | grep -q -- "--instance-group pi-sessions"
}

reached=
cut=
until [ -S "$sock" ]; do
  if old; then
    reached=old
    break
  fi
  now=$(up)
  [ "$now" -lt $((started + 1680)) ] || break
  sleep 0.2
done

if [ -S "$sock" ]; then
  # kitten's words are kept to tell its wait running out from a launch
  # turned down, and reach the journal whether it opened the window or not.
  said=$(kitten @ --to "unix:$sock" launch --response-timeout 60 --type=os-window \
    --os-window-class "$class" --cwd "$dir" --env "MYCELIUM_APP_ID=$class" "$@" \
    2>&1 > /dev/null)
  launched=$?
  [ -z "$said" ] || printf '%s\n' "$said" >&2
  [ "$launched" -ne 0 ] || exit 0
  # Its own last words, never words of the launch's it may repeat.
  case $(printf '%s\n' "$said" | tail -n 1) in
    *"Timed out waiting for a response from kitty"*)
      echo "sessions: the sessions kitty didn't answer for $class in a minute" >&2
      ;;
    *) echo "sessions: the sessions kitty opened no $class" >&2 ;;
  esac
elif [ "$reached" = old ]; then
  systemd-run --user --scope --collect --quiet --slice=agents.slice -- \
    env "MYCELIUM_APP_ID=$class" kitty -1 --instance-group pi-sessions --class "$class" \
    --directory "$dir" "$@" && exit 0
  echo "sessions: the sessions kitty opened no $class" >&2
else
  [ -z "$cut" ] ||
    echo "sessions: systemd didn't name the sessions kitty's process within 2 s" >&2
  echo "sessions: no sessions kitty to open $class in" >&2
fi
# Within 5 s, so a shell stalled or starting again leaves no script behind.
timeout 5 qs ipc call sessions untaken "$class" "$asked" > /dev/null 2>&1 ||
  echo "sessions: the shell didn't hear that no kitty took $class" >&2
exit 1
