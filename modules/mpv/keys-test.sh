# shellcheck shell=bash
# Presses mpv's keys, as its input.conf and vi-keys.lua bind them, and reads
# back over its IPC socket what each did: Vim's beside the arrows that do as
# they do, a number before them, 0 and $, gg G and a number before G through
# the list, ]b and [b, yy, ? and F1, Space at once, Z alone, the keys j and
# l displaced, ZZ closing where q, Escape and Space w don't, and the
# launcher's command line's commands, as mpv-command carries them. mpv
# plays nothing to a screen here, so each key is pressed through mpv's own
# keypress command, which goes through the same bindings a key on the
# keyboard does.
# usage: keys-test.sh <config folder> <a clip of 200 seconds, a keyframe a
# second> <two subtitle files>, with a wl-copy on the config's path that
# writes what it's given to ./copied, and a stand-in launcher-say on the
# path that writes what it's asked to $ASKED. It leaves ./passed once
# every key did what it should.
set -euo pipefail

# The clip three times over, as a list of three.
config=$1 clip=$2 one=$3 two=$4
mpv --config-dir="$config" --vo=null --ao=null --force-window=no --pause --keep-open=yes \
  --save-position-on-quit=no --load-stats-overlay=no --input-ipc-server="$PWD/mpv.sock" \
  --sub-file="$one" --sub-file="$two" "$clip" "$clip" "$clip" > said 2>&1 &
mpv=$!
trap 'kill $mpv 2>/dev/null || true' EXIT

never() {
  echo "never: $1" >&2
  cat said >&2
  exit 1
}

# Before a fresh mpv makes its socket, socat goes without reading what
# printf writes, which is no matter.
asked() {
  { printf '{"command":%s}\n' "$1"; } 2> /dev/null | socat -t 2 - "UNIX-CONNECT:$PWD/mpv.sock" 2> /dev/null
}
# A property mpv has, a time in whole milliseconds, and any other number as
# jq writes it once it's reckoned with, 1.1 rather than mpv's 1.100000.
property() {
  asked "[\"get_property\",\"$1\"]" | jq -r --arg name "$1" 'select(has("error")) | .data
    | if type != "number" then .
      elif ($name | test("time|loop")) then . * 1000 | round
      else . + 0 end'
}
# Keys pressed one after another, over one connection, so a pair's second
# comes well inside the 300 ms its first waits.
keys() {
  for key in "$@"; do
    printf '{"command":["keypress","%s"]}\n' "$key"
  done | socat -t 2 - "UNIX-CONNECT:$PWD/mpv.sock" > /dev/null 2>&1
}
at() { asked "[\"seek\",\"$1\",\"absolute\",\"exact\"]" > /dev/null; }

# Waits up to a minute for a property to read that, saying what it read
# when it never does.
reads() {
  for _ in $(seq 600); do
    if [ "$(property "$1")" = "$2" ]; then return 0; fi
    sleep 0.1
  done
  never "$3 to leave $1 at $2, not $(property "$1")"
}
from() {
  at "$1"
  reads time-pos "${1}000" "a seek to $1 seconds"
}

for _ in $(seq 600); do
  if [ -n "$(property duration)" ]; then break; fi
  sleep 0.1
done
[ -n "$(property duration)" ] || never "mpv to open the clip"

# From a hundred seconds in, each letter lands where its arrow does: j and
# Down on, as j goes on through a file, and k and Up back.
twins() {
  arrow=$1 letter=$2 lands=$3
  from 100
  keys "$arrow"
  reads time-pos "$lands" "$arrow"
  from 100
  keys "$letter"
  reads time-pos "$lands" "$letter, as $arrow"
}
twins LEFT h 95000
twins RIGHT l 105000
twins DOWN j 160000
twins UP k 40000

# A number before a key does it that many times, an arrow's as its
# letter's.
from 100
keys 3 l
reads time-pos 115000 3l
from 160
keys 2 k
reads time-pos 40000 2k
from 100
keys 3 RIGHT
reads time-pos 115000 "3 and Right"

# A number goes 300 ms on with nothing more, choosing nothing, as no
# number chooses from the list, and Escape stops one rather than leaving
# full screen. Neither leaves a number for the key after.
from 100
keys 2
sleep 1
reads playlist-pos 0 "a number alone"
keys l
reads time-pos 105000 "l after a number had gone"
from 100
keys 3 ESC l
reads time-pos 105000 "l after Escape stopped a number"

# 0 goes to the start and $ to the end, where keep-open holds the last
# frame.
from 100
keys 0
reads time-pos 0 0
keys '$'
for _ in $(seq 600); do
  if [ "$(property time-pos)" -ge 198000 ]; then break; fi
  sleep 0.1
done
[ "$(property time-pos)" -ge 198000 ] || never "\$ to the end, not $(property time-pos)"

# [ and ] alone change the speed, a moment on, as mpv has them.
keys ']'
reads speed 1.1 "] alone"
keys BS
reads speed 1 "Backspace"
keys '['
for _ in $(seq 600); do
  case $(property speed) in 0.9*) break ;; esac
  sleep 0.1
done
case $(property speed) in 0.9*) ;; *) never "[ alone to slow it, not $(property speed)" ;; esac
keys BS
reads speed 1 "Backspace again"

# J goes to the next subtitles, as j did, and L sets where a loop starts,
# as l did.
reads sid 1 "the first subtitles to be chosen"
keys J
reads sid 2 "J"
from 100
reads ab-loop-a no "nothing to loop"
keys L
reads ab-loop-a 100000 "L"

# yy copies the frame as a picture, a PNG, and y alone waits for what it's
# to copy, as in Vim, and copies nothing.
copied() {
  for _ in $(seq 600); do
    if [ -s copied ] && [ "$(head -c 4 copied | tail -c 3)" = PNG ]; then return 0; fi
    sleep 0.1
  done
  never "$1 to copy the frame as a PNG"
}
keys y y
copied yy
rm copied
keys y
sleep 1
if [ -e copied ]; then never "y alone to copy nothing"; fi

# ? asks the launcher for the keys, and F1 as well. Space leads nothing:
# before a ? it plays on at once, and the ? is heard alone.
asks() {
  rm -f "$ASKED"
  keys "$@"
  for _ in $(seq 600); do
    if grep -qx 'collection shortcuts' "$ASKED" 2> /dev/null; then return; fi
    sleep 0.1
  done
  never "$* to ask the launcher for the keys"
}
asks '?'
asks F1
asks SPACE '?'
reads pause false "Space before ? to play on"
keys SPACE
reads pause true "Space to pause again"

# : asks the launcher for its command line over this mpv, by the socket
# mpv-command carries what's chosen back over.
rm -f "$ASKED"
keys :
for _ in $(seq 600); do
  if grep -qxF "command-line mpv $PWD/mpv.sock" "$ASKED" 2> /dev/null; then break; fi
  sleep 0.1
done
grep -qxF "command-line mpv $PWD/mpv.sock" "$ASKED" 2> /dev/null \
  || never ": to ask the launcher for its command line"

# Z alone, a moment on, delays the subtitles, as mpv has it.
reads sub-delay 0 "the subtitles undelayed"
keys Z
reads sub-delay 0.1 "Z alone"

# ]b and [b play the next and the one before in the list, and with a
# number that many; gg and G the first and the last, and a number before G
# that one.
reads playlist-pos 0 "the first in the list to play"
keys ']' b
reads playlist-pos 1 "]b"
keys '[' b
reads playlist-pos 0 "[b"
keys 2 ']' b
reads playlist-pos 2 "2]b"
keys g g
reads playlist-pos 0 gg
keys G
reads playlist-pos 2 G
keys 2 G
reads playlist-pos 1 2G
keys g g
reads playlist-pos 0 "gg again"

# f fills the screen and Escape leaves it; Space plays on and pauses.
keys f
reads fullscreen true f
keys ESC
reads fullscreen false Escape
keys SPACE
reads pause false Space
keys SPACE
reads pause true "Space again"

# The launcher's command line, as mpv-command carries it: a time goes
# there, :set turns pause off, on and over as Vim's :set does and sets a
# property by its value, :y copies the frame, and a command mpv hasn't
# does nothing.
told() { mpv-command "$PWD/mpv.sock" "$@"; }
told goto 1:30
reads time-pos 90000 ":1:30"
told goto 45
reads time-pos 45000 ":45"
told goto 1:x
sleep 1
reads time-pos 45000 ":1:x to go nowhere"
told set nopause
reads pause false ":set nopause"
told set pause
reads pause true ":set pause"
told set 'pause!'
reads pause false ":set pause!"
told set pause
reads pause true ":set pause again"
told set volume=40
reads volume 40 ":set volume=40"
rm -f copied
told yank
copied ":y"
told nonsense

# Waits up to a minute for this mpv to close, saying what didn't close it.
closed() {
  for _ in $(seq 600); do
    if ! kill -0 "$mpv" 2> /dev/null; then return 0; fi
    sleep 0.1
  done
  never "$1 to close"
}
# q, Q, Escape at rest, and Space w, close nothing: a second on, the player
# is still there, since Space leads nothing here.
open_after() {
  keys "$@"
  sleep 1
  kill -0 "$mpv" 2> /dev/null || never "$* to leave it open"
}
open_after q
open_after Q
open_after ESC
open_after SPACE w
# ZZ closes it.
keys Z Z
closed ZZ

# :q closes one given no socket, over the one vi-keys.lua gives it,
# named for its process.
mpv --config-dir="$config" --vo=null --ao=null --force-window=no --idle=yes \
  --save-position-on-quit=no --load-stats-overlay=no > said 2>&1 &
mpv=$!
for _ in $(seq 600); do
  if [ -S "$XDG_RUNTIME_DIR/mpv-$mpv.sock" ]; then break; fi
  sleep 0.1
done
[ -S "$XDG_RUNTIME_DIR/mpv-$mpv.sock" ] || never "vi-keys.lua to give mpv a socket"
mpv-command "$XDG_RUNTIME_DIR/mpv-$mpv.sock" quit
closed ":q"

# A program that runs mpv with Escape of its own, forced, as the camera's
# viewfinder does, always gets it, whichever script mpv loads first: of
# two forced bindings the last loaded wins, so it's tried several times.
cat > own-escape.lua << 'lua'
mp.add_forced_key_binding('ESC', 'own-escape', function()
  io.open('escaped', 'w'):close()
end)
lua
# How many of the two scripts' Escapes mpv has bound.
escapes() {
  asked '["get_property","input-bindings"]' \
    | jq '[.data // [] | .[] | select(.cmd | test("own-escape|vi-ESC"))] | length'
}
for time in 1 2 3 4 5 6; do
  rm -f escaped
  mpv --config-dir="$config" --vo=null --ao=null --force-window=no --idle=yes \
    --save-position-on-quit=no --load-stats-overlay=no --input-ipc-server="$PWD/mpv.sock" \
    --script="$PWD/own-escape.lua" > said 2>&1 &
  mpv=$!
  for _ in $(seq 600); do
    if [ "$(escapes)" = 2 ]; then break; fi
    sleep 0.1
  done
  [ "$(escapes)" = 2 ] || never "both scripts to bind Escape, time $time"
  keys ESC
  for _ in $(seq 50); do
    if [ -e escaped ]; then break; fi
    sleep 0.1
  done
  [ -e escaped ] || never "a program's own forced Escape to have it, time $time"
  kill "$mpv"
  wait "$mpv" 2> /dev/null || true
done
touch passed
