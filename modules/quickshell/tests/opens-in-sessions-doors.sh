# shellcheck shell=bash
# Runs modules/quickshell/opens-in-sessions.sh through each door it can
# take, over stand-ins for kitty's kitten, systemd-run, systemctl, sleep,
# timeout, date and the uptime cut reads, and the shell's qs: its socket,
# a socket that comes while it
# waits, the unit's old kitty, and neither, which opens nothing and tells
# the shell so, as it does when the kitty it asks turns the launch down or
# can't be reached, or the old one opens nothing, and says so when the
# shell doesn't hear it. The old kitty is a real process with that kitty's
# words on its command line, since the script reads /proc, which no
# stand-in can be. What the script
# tells the shell is held to what Sessions.qml's handler takes, so the two
# can't drift apart while both stand-ins pass, and the words it reads as
# kitten's wait running out are what the packaged kitten says then.
# usage: opens-in-sessions-doors.sh <sh> <opens-in-sessions.sh> <python3> <Sessions.qml> <kitten>
set -euo pipefail

sh=$1
script=$2
python=$3
sessions=$4
kitten=$5
# kitten's words for its wait for an async command's answer running out
# (its tools/cmd/at/main.go), which a launch is (kitty/rc/launch.py).
timed_out="Timed out waiting for a response from kitty"
real_sleep=$(type -P sleep)
real_cut=$(type -P cut)
real_timeout=$(type -P timeout)
work=$(mktemp -d)
stand_ins=$work/bin
mkdir -p "$stand_ins"
failed=0

# Each stand-in says what it was asked, one line, into asked.
# Each fails while a file of its name and -fails is there. A kitty's
# launch failing counts as a wait, as the real one's answer can take a
# minute, so the clock tells a time read as the script started from one
# read once it has failed.
# A kitty's launch says too whether what it opens would inherit the
# script's first start, which it never should. While kitten-times-out is
# there, kitten says its wait for the kitty's answer ran out, in its own
# words, after a wait; while kitten-warns is there, it warns and opens the
# window; a launch that opens one says the new window's id, as the real
# one does; and a launch it turns down says so in words of its own.
for program in kitten systemd-run qs; do
  printf '#!%s\necho "%s $*" >> "%s/asked"\n' "$BASH" "$program" "$work" \
    > "$stand_ins/$program"
  if [ "$program" != qs ]; then
    # shellcheck disable=SC2016 # The stand-in it writes expands it, when run.
    printf '%s || echo "%s inherited" >> "%s/inherited"\n' \
      '[ -z "${OPENS_IN_SESSIONS_ASKED+x}${OPENS_IN_SESSIONS_STARTED+x}" ]' \
      "$program" "$work" >> "$stand_ins/$program"
  fi
  if [ "$program" = kitten ]; then
    printf '[ ! -f "%s/kitten-times-out" ] || { echo . >> "%s/waits"; %s; exit 1; }\n' \
      "$work" "$work" "echo '$timed_out' >&2" >> "$stand_ins/$program"
    printf '[ ! -f "%s/kitten-warns" ] || echo "kitten: a warning" >&2\n' "$work" \
      >> "$stand_ins/$program"
  fi
  if [ "$program" = kitten ]; then
    printf '[ -f "%s/kitten-fails" ] || { echo 42; exit; }\n' "$work" >> "$stand_ins/$program"
  else
    printf '[ ! -f "%s/%s-fails" ] && exit\n' "$work" "$program" >> "$stand_ins/$program"
  fi
  if [ "$program" != qs ]; then
    printf 'echo . >> "%s/waits"\n' "$work" >> "$stand_ins/$program"
  fi
  if [ "$program" = kitten ]; then
    echo 'echo "kitten: the launch was turned down" >&2' >> "$stand_ins/$program"
  fi
  echo "exit 1" >> "$stand_ins/$program"
done
# The journal, as the script's own output when niri gave it none: says
# what it was asked and runs the rest, its output into said, or, while
# systemd-cat-fails is there, fails without running it, as systemd-cat
# does when it can't reach the journal. Each counts as a wait, so a
# rerun that read the clock afresh would tell a later time.
printf '#!%s\necho "systemd-cat $*" >> "%s/asked"\necho . >> "%s/waits"\n' \
  "$BASH" "$work" "$work" > "$stand_ins/systemd-cat"
printf '[ ! -f "%s/systemd-cat-fails" ] || exit 1\nshift\nexec "$@" 2>> "%s/said"\n' \
  "$work" "$work" >> "$stand_ins/systemd-cat"
# The shell's own clock, in ms as the shell's handler takes it: asked any
# other way, it answers what it was asked, which no case wants. It moves
# 200 ms with each wait, the script's or a launch's, so the time the shell
# is told is 1700000000000 only if the script read it as it started,
# never as it gave up, which would clear a press made meanwhile. While
# clock-back is there, every read after the first is an hour earlier, as
# a time sync stepping the clock back.
cat > "$stand_ins/date" << EOF
#!$BASH
now=\$((1700000000000 + \$(wc -l < "$work/waits") * 200))
if [ -f "$work/clock-back" ] && [ -f "$work/clock-read" ]; then
  now=\$((now - 3600000))
fi
touch "$work/clock-read"
if [ "\$*" = +%s%3N ]; then
  echo "\$now"
else
  echo "date \$*"
fi
EOF
# The time since boot, as /proc/uptime's first field, which no time sync
# steps: 200 ms more with each wait, as the clock, from a fraction of .08,
# which a shell reading it as an octal number fails on. Any other cut is
# cut.
cat > "$stand_ins/cut" << EOF
#!$BASH
if [ "\${!#}" = /proc/uptime ]; then
  up=\$((1234508 + \$(wc -l < "$work/waits") * 20))
  printf '%d.%02d\n' \$((up / 100)) \$((up % 100))
else
  exec "$real_cut" "\$@"
fi
EOF
# The unit's main process, as main-pid names it, taking a second to answer
# while systemctl-slow is there, as under a cold boot's load; and while
# systemctl-hangs is there, half a minute once 16.8 s have passed, so the
# last look the script may begin hangs, as a user manager stalling.
cat > "$stand_ins/systemctl" << EOF
#!$BASH
if [ -f "$work/systemctl-slow" ]; then
  printf '.\n.\n.\n.\n.\n' >> "$work/waits"
fi
if [ -f "$work/systemctl-hangs" ] && [ "\$(wc -l < "$work/waits")" -ge 84 ]; then
  for _ in \$(seq 150); do echo . >> "$work/waits"; done
fi
cat "$work/main-pid"
EOF
# coreutils' timeout, running what it was given, but ending a hung
# systemctl at its limit, as the real one does, so a case's clock moves by
# the limit and not by the hang.
cat > "$stand_ins/timeout" << EOF
#!$BASH
limit=\$1
shift
if [ "\$1" = systemctl ] && [ -f "$work/systemctl-hangs" ] &&
  [ "\$(wc -l < "$work/waits")" -ge 84 ]; then
  for _ in \$(seq \$((limit * 5))); do echo . >> "$work/waits"; done
  exit 124
fi
exec "\$@"
EOF
# Each wait counted, and the socket made at the wait came-at names.
cat > "$stand_ins/sleep" << EOF
#!$BASH
echo . >> "$work/waits"
if [ -f "$work/came-at" ] && [ "\$(wc -l < "$work/waits")" -eq "\$(cat "$work/came-at")" ]; then
  "$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
fi
EOF
chmod +x "$stand_ins"/*

old=""
trap '[ -z "$old" ] || kill "$old"; rm -rf "$work"' EXIT

fresh() {
  rm -f "$work/came-at" "$work/kitty.sock" "$work"/*-fails "$work"/kitten-* \
    "$work"/systemctl-* "$work"/clock-* "$work/pwned"
  : > "$work/asked"
  : > "$work/inherited"
  : > "$work/said"
  : > "$work/waits"
  echo 0 > "$work/main-pid"
}

# How the script ended is kept in status, so one that fails is said by the
# case it fails rather than ending them all; and one that runs on is ended
# in half a minute, 124, as no case takes that long with the stand-ins.
opens() {
  status=0
  HOME=/home/joel PATH="$stand_ins:$PATH" "$real_timeout" 30 "$sh" "$script" \
    "$work/kitty.sock" "$@" 2> "$work/said" || status=$?
}
# As niri starts it, with no output (its spawning.rs gives a child none).
opens_unheard() {
  HOME=/home/joel PATH="$stand_ins:$PATH" "$real_timeout" 30 "$sh" "$script" \
    "$work/kitty.sock" "$@" 2> /dev/null || true
}

# What a case wanted, against what the file it names holds.
expect() {
  local what=$1 want=$2 got
  got=$(< "$work/$3")
  if [ "$got" != "$want" ]; then
    echo "FAIL $what"
    echo "  wanted: $want"
    echo "  got:    $got"
    failed=1
  fi
}

# How many times a case's script waited, against what it wanted.
waited() {
  local what=$1 want=$2 got
  got=$(wc -l < "$work/waits")
  if [ "$got" -ne "$want" ]; then
    echo "FAIL $what: waited $got times, not $want"
    failed=1
  fi
}

# How long a case's script took by the clock, at most what it wanted, in
# waits of 200 ms: a bound, where the count of looks it fits isn't the
# case's point.
waited_at_most() {
  local what=$1 most=$2 got
  got=$(wc -l < "$work/waits")
  if [ "$got" -gt "$most" ]; then
    echo "FAIL $what: took $got waits of 200 ms, past $most"
    failed=1
  fi
}

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
opens pi-session.7 --directory /src/reef zsh -lic 'mycelium attach'
# Its answer waited for a minute, so a launch kitty turns down is said,
# while one slow to draw is waited on past the shell's own patience.
expect "a kitty listening is asked over its socket, in the folder named, waiting its answer" \
  "kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-session.7 --cwd /src/reef --env MYCELIUM_APP_ID=pi-session.7 \
zsh -lic mycelium attach" asked
expect "a launch that opens its window quietly puts nothing in the journal" "" said

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
opens pi-spawned.8 pi
expect "one naming no folder opens at home" \
  "kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-spawned.8 --cwd /home/joel --env MYCELIUM_APP_ID=pi-spawned.8 pi" asked

fresh
echo 3 > "$work/came-at"
opens pi-session.9 pi
expect "a kitty starting again is waited on until its socket comes" \
  "kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-session.9 --cwd /home/joel --env MYCELIUM_APP_ID=pi-session.9 pi" asked
waited "it waits only until then" 3

fresh
# A kitty from before the unit listened, as it was started: one process with
# those words on its command line, holding none of this case's output open.
"$python" -c 'import time; time.sleep(300)' --instance-group pi-sessions > /dev/null 2>&1 &
old=$!
# Until it has started, its command line is still this one's.
until tr '\0' ' ' < "/proc/$old/cmdline" | grep -q -- "--instance-group"; do
  "$real_sleep" 0.05
done
echo "$old" > "$work/main-pid"
opens pi-session.10 --directory /src/kelp pi
expect "the unit's own old kitty is reached as its single instance" \
  "systemd-run --user --scope --collect --quiet --slice=agents.slice -- env \
MYCELIUM_APP_ID=pi-session.10 kitty -1 --instance-group pi-sessions --class pi-session.10 \
--directory /src/kelp pi" asked
waited "it isn't waited on" 0
# The old kitty failing to open it, the shell is told none took it.
: > "$work/asked"
touch "$work/systemd-run-fails"
opens pi-hosted.s-old pi
expect "an old kitty that opens nothing is asked once, and the shell is told none took it" \
  "systemd-run --user --scope --collect --quiet --slice=agents.slice -- env \
MYCELIUM_APP_ID=pi-hosted.s-old kitty -1 --instance-group pi-sessions --class pi-hosted.s-old \
--directory /home/joel pi
qs ipc call sessions untaken pi-hosted.s-old 1700000000000" asked
expect "the journal says it opened nothing" \
  "sessions: the sessions kitty opened no pi-hosted.s-old" said
kill "$old"
old=""

fresh
# Another program's kitty as the unit's process, which isn't its old one.
"$python" -c 'import time; time.sleep(300)' > /dev/null 2>&1 &
old=$!
echo "$old" > "$work/main-pid"
opens pi-hosted.s-11 pi
if [ "$status" -ne 1 ]; then
  echo "FAIL with neither, it ended $status, not 1"
  failed=1
fi
expect "with neither, no kitty is asked, and the shell is told none took it" \
  "qs ipc call sessions untaken pi-hosted.s-11 1700000000000" asked
waited "it stopped looking once 17 s had passed" 85
expect "the journal says why" "sessions: no sessions kitty to open pi-hosted.s-11 in" said
kill "$old"
old=""

# However long each look at the unit takes, a second under a cold boot's
# load or a hang its timeout ends in the last look it may begin, and with
# the clock stepped back too, it gives up within 19 s of its start by the
# stand-ins' clock, which charges nothing for starting a program, leaving
# a second of the 20 s the shell's answerPatience counts on from it; and
# tells the shell when it started. The neither case's exact count holds
# when the last look may begin, and these that each look is cut.
for slow in systemctl-slow systemctl-hangs "clock-back systemctl-hangs"; do
  fresh
  "$python" -c 'import time; time.sleep(300)' > /dev/null 2>&1 &
  old=$!
  echo "$old" > "$work/main-pid"
  for flag in $slow; do
    touch "$work/$flag"
  done
  opens pi-hosted.s-15 pi
  if [ "$status" -ne 1 ]; then
    echo "FAIL with $slow, it ended $status, not 1"
    failed=1
  fi
  waited_at_most "with $slow, it gives up within 19 s" 95
  expect "with $slow, the shell is told when it started" \
    "qs ipc call sessions untaken pi-hosted.s-15 1700000000000" asked
  expect "with $slow, the journal says why" \
    "sessions: no sessions kitty to open pi-hosted.s-15 in" said
  kill "$old"
  old=""
done

fresh
"$python" -c 'import time; time.sleep(300)' > /dev/null 2>&1 &
old=$!
echo "$old" > "$work/main-pid"
# A start it inherits that isn't a time, from whatever set its
# environment, gives way to the script's own: the shell is told its own
# time, and it waits its own 17 s. The start is what the wait's arithmetic
# reads, so it's what the pwned file proves is never evaluated.
export OPENS_IN_SESSIONS_ASKED="x[\$(touch $work/pwned)]"
export OPENS_IN_SESSIONS_STARTED="x[\$(touch $work/pwned)]"
opens pi-hosted.s-17 pi
unset OPENS_IN_SESSIONS_ASKED OPENS_IN_SESSIONS_STARTED
if [ -e "$work/pwned" ]; then
  echo "FAIL an inherited start ran as code"
  failed=1
fi
expect "an inherited start that isn't a time gives way to its own" \
  "qs ipc call sessions untaken pi-hosted.s-17 1700000000000" asked
waited "an inherited start that isn't a time waits from its own" 85
# Nor does a start that is a number but later than now, or too long for
# the shell's arithmetic, as left from a longer boot, hold it past its
# own 17 s; one earlier, as its own run under the journal leaves it, is
# taken, with the time beside it.
for start in 99999999999 99999999999999999999; do
  fresh
  echo "$old" > "$work/main-pid"
  export OPENS_IN_SESSIONS_ASKED=1699999999000 OPENS_IN_SESSIONS_STARTED=$start
  opens pi-hosted.s-17 pi
  unset OPENS_IN_SESSIONS_ASKED OPENS_IN_SESSIONS_STARTED
  expect "an inherited start of $start gives way to its own, the time with it" \
    "qs ipc call sessions untaken pi-hosted.s-17 1700000000000" asked
  waited "an inherited start of $start waits from its own" 85
done
fresh
echo "$old" > "$work/main-pid"
export OPENS_IN_SESSIONS_ASKED=1699999999000 OPENS_IN_SESSIONS_STARTED=1234408
opens pi-hosted.s-17 pi
unset OPENS_IN_SESSIONS_ASKED OPENS_IN_SESSIONS_STARTED
expect "an inherited start a second earlier is taken, with its time" \
  "qs ipc call sessions untaken pi-hosted.s-17 1699999999000" asked
waited "it waits from that start" 80
# Started as niri starts it, its run under the journal waits from its
# first start, not from its own, which comes two of the journal's waits
# later.
fresh
echo "$old" > "$work/main-pid"
opens_unheard pi-hosted.s-19 pi
expect "one started with no output tells the shell its first start" \
  "systemd-cat --identifier=opens-in-sessions true
systemd-cat --identifier=opens-in-sessions sh $script $work/kitty.sock pi-hosted.s-19 pi
qs ipc call sessions untaken pi-hosted.s-19 1700000000000" asked
waited "and waits from its first start" 85
kill "$old"
old=""

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
# A kitty that turns the launch down, or a socket it no longer listens on,
# as after a crash; and a shell that doesn't hear the give-up, which is
# said too. The shell is told when the script started, not when it gave up.
touch "$work/kitten-fails" "$work/qs-fails"
opens pi-hosted.s-12 pi
if [ "$status" -ne 1 ]; then
  echo "FAIL with a kitty that opens nothing, it ended $status, not 1"
  failed=1
fi
expect "a kitty that opens nothing is asked once, and the shell is told none took it" \
  "kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-hosted.s-12 --cwd /home/joel --env MYCELIUM_APP_ID=pi-hosted.s-12 pi
qs ipc call sessions untaken pi-hosted.s-12 1700000000000" asked
expect "the journal says it opened nothing, in kitten's words and its own, and that the shell \
didn't hear" \
  "kitten: the launch was turned down
sessions: the sessions kitty opened no pi-hosted.s-12
sessions: the shell didn't hear that no kitty took pi-hosted.s-12" said

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
# A kitty that doesn't answer within kitten's minute may still open the
# window, so it's said as not answering, never as taking none, beside
# kitten's own words; the shell's ask is over all the same.
touch "$work/kitten-times-out"
opens pi-hosted.s-16 pi
if [ "$status" -ne 1 ]; then
  echo "FAIL with a kitty that doesn't answer, it ended $status, not 1"
  failed=1
fi
expect "a kitty that doesn't answer is asked once, and the shell is told" \
  "kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-hosted.s-16 --cwd /home/joel --env MYCELIUM_APP_ID=pi-hosted.s-16 pi
qs ipc call sessions untaken pi-hosted.s-16 1700000000000" asked
expect "the journal says it didn't answer, in kitten's words and its own" \
  "$timed_out
sessions: the sessions kitty didn't answer for pi-hosted.s-16 in a minute" said

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
# What kitten says of a launch that opened its window reaches the journal
# too, and the shell is told nothing.
touch "$work/kitten-warns"
opens pi-hosted.s-18 pi
if [ "$status" -ne 0 ]; then
  echo "FAIL with a kitty that warns and opens it, it ended $status, not 0"
  failed=1
fi
expect "a kitty that warns and opens it is asked once, and the shell told nothing" \
  "kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-hosted.s-18 --cwd /home/joel --env MYCELIUM_APP_ID=pi-hosted.s-18 pi" asked
expect "the journal hears kitten's warning" "kitten: a warning" said

# The packaged kitten, asking a socket that takes the launch and never
# answers, ends failed and says its wait ran out in the words the script
# looks for, on the stream it keeps, so a kitty that rewords them fails
# here rather than in the journal.
"$python" -c 'import socket, sys, time
listening = socket.socket(socket.AF_UNIX)
listening.bind(sys.argv[1])
listening.listen()
taken = [listening.accept() for _ in range(2)]
time.sleep(30)' "$work/silent.sock" > /dev/null 2>&1 &
silent=$!
until [ -S "$work/silent.sock" ]; do
  "$real_sleep" 0.05
done
kitten_status=0
kitten_said=$(HOME=$work timeout 20 "$kitten" @ --to "unix:$work/silent.sock" launch \
  --response-timeout 1 --type=os-window true 2>&1 > /dev/null) || kitten_status=$?
kill "$silent" 2> /dev/null || true
if [ "$kitten_status" -eq 0 ] || [ "$kitten_status" -eq 124 ]; then
  echo "FAIL the packaged kitten, never answered, ended $kitten_status"
  failed=1
fi
case $kitten_said in
  *"$timed_out"*) ;;
  *)
    echo "FAIL the packaged kitten, never answered, didn't say: $timed_out"
    echo "  it said: $kitten_said"
    failed=1
    ;;
esac

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
# As niri starts it, with no output: once the journal is found to take
# its words, it runs again once with its output there, telling the shell
# when it was first started.
touch "$work/kitten-fails"
opens_unheard pi-hosted.s-13 pi
expect "one started with no output says why in the journal, told its first start" \
  "systemd-cat --identifier=opens-in-sessions true
systemd-cat --identifier=opens-in-sessions sh $script $work/kitty.sock pi-hosted.s-13 pi
kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-hosted.s-13 --cwd /home/joel --env MYCELIUM_APP_ID=pi-hosted.s-13 pi
qs ipc call sessions untaken pi-hosted.s-13 1700000000000" asked
expect "the journal hears it opened nothing" \
  "kitten: the launch was turned down
sessions: the sessions kitty opened no pi-hosted.s-13" said
expect "nothing it launches inherits its first start" "" inherited

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
# A time inherited from whatever else set its environment isn't taken for
# its own run's, so it still runs again with its output in the journal.
touch "$work/kitten-fails"
export OPENS_IN_SESSIONS_ASKED=x
opens_unheard pi-hosted.s-20 pi
unset OPENS_IN_SESSIONS_ASKED
expect "one given a time not its own still says why in the journal" \
  "systemd-cat --identifier=opens-in-sessions true
systemd-cat --identifier=opens-in-sessions sh $script $work/kitty.sock pi-hosted.s-20 pi
kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-hosted.s-20 --cwd /home/joel --env MYCELIUM_APP_ID=pi-hosted.s-20 pi
qs ipc call sessions untaken pi-hosted.s-20 1700000000000" asked

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
# With a journal that won't take its words, as systemd-cat finds when it
# can't make its stream, it runs on as it was, never ending there.
touch "$work/kitten-fails" "$work/systemd-cat-fails"
opens_unheard pi-hosted.s-14 pi
expect "one whose journal won't take its words still asks, and tells the shell" \
  "systemd-cat --identifier=opens-in-sessions true
kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-hosted.s-14 --cwd /home/joel --env MYCELIUM_APP_ID=pi-hosted.s-14 pi
qs ipc call sessions untaken pi-hosted.s-14 1700000000000" asked

# The shell's handler takes what the script tells it: its target, the
# function's name, and an argument for each word after it, which it
# passes on to the shell's own untaken in the order it took them, outside
# a comment. Ending 2 says the order is wrong, 1 that there's no handler.
handler=0
"$python" - "$sessions" << 'EOF' || handler=$?
import re
import sys

source = open(sys.argv[1], encoding="utf-8").read()
for handler in re.finditer(r"\bIpcHandler\s*\{", source):
    depth, at = 1, handler.end()
    while depth and at < len(source):
        depth += {"{": 1, "}": -1}.get(source[at], 0)
        at += 1
    block = source[handler.end():at]
    taken = re.search(r"function untaken\((\w+): string, (\w+): string\)[^{]*\{", block)
    if re.search(r'^\s*target:\s*"sessions"\s*$', block, re.M) and taken:
        named, asked = taken.groups()
        depth, end = 1, taken.end()
        while depth and end < len(block):
            depth += {"{": 1, "}": -1}.get(block[end], 0)
            end += 1
        body = re.sub(r"/\*.*?\*/", "", block[taken.end():end], flags=re.S)
        body = re.sub(r"//[^\n]*", "", body)
        passed = rf"\broot\.untaken\(\s*{named}\s*,\s*{asked}\s*\)"
        sys.exit(0 if re.search(passed, body) else 2)
sys.exit(1)
EOF
case $handler in
  0) ;;
  2)
    echo "FAIL the shell's sessions handler doesn't pass untaken's words on in order"
    failed=1
    ;;
  *)
    echo "FAIL the shell's sessions handler takes no untaken of an app id and a time"
    failed=1
    ;;
esac

exit "$failed"
