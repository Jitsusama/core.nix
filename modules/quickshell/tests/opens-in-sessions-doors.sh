# shellcheck shell=bash
# Runs modules/quickshell/opens-in-sessions.sh through each door it can
# take, over stand-ins for kitty's kitten, systemd-run, systemctl and
# sleep: its socket, a socket that comes while it waits, the unit's old
# kitty, and neither, which opens nothing. The old kitty is a real
# process with that kitty's words on its command line, since the script
# reads /proc, which no stand-in can be.
# usage: opens-in-sessions-doors.sh <sh> <opens-in-sessions.sh> <python3>
set -euo pipefail

sh=$1
script=$2
python=$3
real_sleep=$(type -P sleep)
work=$(mktemp -d)
stand_ins=$work/bin
mkdir -p "$stand_ins"
failed=0

# Each stand-in says what it was asked, one line, into asked.
for program in kitten systemd-run; do
  printf '#!%s\necho "%s $*" >> "%s/asked"\n' "$BASH" "$program" "$work" > "$stand_ins/$program"
done
# The unit's main process, as main-pid names it.
printf '#!%s\ncat "%s/main-pid"\n' "$BASH" "$work" > "$stand_ins/systemctl"
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
  rm -f "$work/came-at" "$work/kitty.sock"
  : > "$work/asked"
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

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
opens pi-session.7 --directory /src/reef zsh -lic 'mycelium attach'
expect "a kitty listening is asked over its socket, in the folder named" \
  "kitten @ --to unix:$work/kitty.sock launch --type=os-window --os-window-class pi-session.7 \
--cwd /src/reef --env MYCELIUM_APP_ID=pi-session.7 zsh -lic mycelium attach" asked

fresh
"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' "$work/kitty.sock"
opens pi-spawned.8 pi
expect "one naming no folder opens at home" \
  "kitten @ --to unix:$work/kitty.sock launch --type=os-window --os-window-class pi-spawned.8 \
--cwd /home/joel --env MYCELIUM_APP_ID=pi-spawned.8 pi" asked

fresh
echo 3 > "$work/came-at"
opens pi-session.9 pi
expect "a kitty starting again is waited on until its socket comes" \
  "kitten @ --to unix:$work/kitty.sock launch --type=os-window --os-window-class pi-session.9 \
--cwd /home/joel --env MYCELIUM_APP_ID=pi-session.9 pi" asked
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
kill "$old"
old=""

fresh
# Another program's kitty as the unit's process, which isn't its old one.
"$python" -c 'import time; time.sleep(300)' > /dev/null 2>&1 &
old=$!
echo "$old" > "$work/main-pid"
opens pi-session.11 pi
if [ "$status" -ne 1 ]; then
  echo "FAIL with neither, it ended $status, not 1"
  failed=1
fi
expect "with neither, nothing is asked" "" asked
waited "it gave up after 20 s of waits" 99
expect "the journal says why" "sessions: no sessions kitty to open pi-session.11 in" said

exit "$failed"
