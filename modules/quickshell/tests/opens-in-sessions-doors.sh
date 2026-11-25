# shellcheck shell=bash
# Runs modules/quickshell/opens-in-sessions.sh through each door it can
# take, over stand-ins for kitty's kitten, systemd-run, systemctl, sleep
# and date, and the shell's qs: its socket, a socket that comes while it
# waits, the unit's old kitty, and neither, which opens nothing and tells
# the shell so, as it does when the kitty it asks turns the launch down or
# can't be reached, or the old one opens nothing, and says so when the
# shell doesn't hear it. The old kitty is a real process with that kitty's
# words on its command line, since the script reads /proc, which no
# stand-in can be. What the script
# tells the shell is held to what Sessions.qml's handler takes, so the two
# can't drift apart while both stand-ins pass.
# usage: opens-in-sessions-doors.sh <sh> <opens-in-sessions.sh> <python3> <Sessions.qml>
set -euo pipefail

sh=$1
script=$2
python=$3
sessions=$4
real_sleep=$(type -P sleep)
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
# there, kitten says its wait for the kitty's answer ran out, in the words
# its at/main.go gives an async command's, after a wait.
for program in kitten systemd-run qs; do
  printf '#!%s\necho "%s $*" >> "%s/asked"\n' "$BASH" "$program" "$work" \
    > "$stand_ins/$program"
  if [ "$program" != qs ]; then
    # shellcheck disable=SC2016 # The stand-in it writes expands it, when run.
    printf '[ -z "${OPENS_IN_SESSIONS_ASKED+x}" ] || echo "%s inherited" >> "%s/inherited"\n' \
      "$program" "$work" >> "$stand_ins/$program"
  fi
  if [ "$program" = kitten ]; then
    printf '[ ! -f "%s/kitten-times-out" ] || { echo . >> "%s/waits"; %s; exit 1; }\n' \
      "$work" "$work" 'echo "Timed out waiting for a response from kitty" >&2' \
      >> "$stand_ins/$program"
  fi
  printf '[ ! -f "%s/%s-fails" ] && exit\n' "$work" "$program" >> "$stand_ins/$program"
  if [ "$program" != qs ]; then
    printf 'echo . >> "%s/waits"\n' "$work" >> "$stand_ins/$program"
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
# The shell's own clock, only when asked in ms, as the shell's handler
# takes it: asked any other way, it answers what it was asked, which no
# case wants. Its clock moves 200 ms with each wait, the script's or a
# launch's, so
# the time the shell is told is 1700000000000 only if the script read it
# as it started, never as it gave up, which would clear a press made
# meanwhile.
cat > "$stand_ins/date" << EOF
#!$BASH
if [ "\$*" != +%s%3N ]; then
  echo "date \$*"
else
  echo \$((1700000000000 + \$(wc -l < "$work/waits") * 200))
fi
EOF
# The unit's main process, as main-pid names it, taking a second to answer
# while systemctl-slow is there, as under a cold boot's load.
cat > "$stand_ins/systemctl" << EOF
#!$BASH
if [ -f "$work/systemctl-slow" ]; then
  printf '.\n.\n.\n.\n.\n' >> "$work/waits"
fi
cat "$work/main-pid"
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
  rm -f "$work/came-at" "$work/kitty.sock" "$work"/*-fails
  : > "$work/asked"
  : > "$work/inherited"
  : > "$work/said"
  : > "$work/waits"
  echo 0 > "$work/main-pid"
}

# How the script ended is kept in status, so one that fails is said by the
# case it fails rather than ending them all.
opens() {
  status=0
  HOME=/home/joel PATH="$stand_ins:$PATH" "$sh" "$script" "$work/kitty.sock" "$@" \
    2> "$work/said" || status=$?
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
# waits of 200 ms.
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
waited "it gave up once its clock said 20 s had passed" 100
expect "the journal says why" "sessions: no sessions kitty to open pi-hosted.s-11 in" said
# Each look at the unit taking a second, as under a cold boot's load, it
# still gives up by 20 s of its clock, not after a hundred looks: at most
# 20 s, then one look and wait past it, and the last look.
: > "$work/waits"
: > "$work/asked"
touch "$work/systemctl-slow"
opens pi-hosted.s-15 pi
waited_at_most "under a slow systemctl, it gives up by its clock" 111
expect "under a slow systemctl, the shell is told when it started" \
  "qs ipc call sessions untaken pi-hosted.s-15 1700000000000" asked
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
expect "the journal says it opened nothing, and that the shell didn't hear" \
  "sessions: the sessions kitty opened no pi-hosted.s-12
sessions: the shell didn't hear that no kitty took pi-hosted.s-12" said

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
# A kitty that doesn't answer within kitten's minute may still open the
# window, so it's said as not answering, never as taking none, beside
# kitten's own words; the shell's ask is over all the same.
touch "$work/kitten-times-out"
opens pi-hosted.s-16 pi
expect "a kitty that doesn't answer is asked once, and the shell is told" \
  "kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-hosted.s-16 --cwd /home/joel --env MYCELIUM_APP_ID=pi-hosted.s-16 pi
qs ipc call sessions untaken pi-hosted.s-16 1700000000000" asked
expect "the journal says it didn't answer, in kitten's words and its own" \
  "Timed out waiting for a response from kitty
sessions: the sessions kitty didn't answer for pi-hosted.s-16 in a minute" said

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
# As niri starts it, with no output (its spawning.rs gives a child none):
# once the journal is found to take its words, it runs again once with
# its output there, telling the shell when it was first started.
touch "$work/kitten-fails"
HOME=/home/joel PATH="$stand_ins:$PATH" "$sh" "$script" "$work/kitty.sock" pi-hosted.s-13 pi \
  2> /dev/null || true
expect "one started with no output says why in the journal, told its first start" \
  "systemd-cat --identifier=opens-in-sessions true
systemd-cat --identifier=opens-in-sessions sh $script $work/kitty.sock pi-hosted.s-13 pi
kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-hosted.s-13 --cwd /home/joel --env MYCELIUM_APP_ID=pi-hosted.s-13 pi
qs ipc call sessions untaken pi-hosted.s-13 1700000000000" asked
expect "the journal hears it opened nothing" \
  "sessions: the sessions kitty opened no pi-hosted.s-13" said
expect "nothing it launches inherits its first start" "" inherited

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
# With a journal that won't take its words, as systemd-cat finds when it
# can't make its stream, it runs on as it was, never ending there.
touch "$work/kitten-fails" "$work/systemd-cat-fails"
HOME=/home/joel PATH="$stand_ins:$PATH" "$sh" "$script" "$work/kitty.sock" pi-hosted.s-14 pi \
  2> /dev/null || true
expect "one whose journal won't take its words still asks, and tells the shell" \
  "systemd-cat --identifier=opens-in-sessions true
kitten @ --to unix:$work/kitty.sock launch --response-timeout 60 --type=os-window \
--os-window-class pi-hosted.s-14 --cwd /home/joel --env MYCELIUM_APP_ID=pi-hosted.s-14 pi
qs ipc call sessions untaken pi-hosted.s-14 1700000000000" asked

# The shell's handler takes what the script tells it: its target, the
# function's name, and an argument for each word after it, which it
# passes on to the shell's own untaken in the order it took them.
if ! "$python" - "$sessions" << 'EOF'
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
        body = block[taken.end():block.find("}", taken.end())]
        passed = rf"\broot\.untaken\(\s*{named}\s*,\s*{asked}\s*\)"
        sys.exit(0 if re.search(passed, body) else 1)
sys.exit(1)
EOF
then
  echo "FAIL the shell's sessions handler takes no untaken of an app id and a time," \
    "passed on in order"
  failed=1
fi

exit "$failed"
