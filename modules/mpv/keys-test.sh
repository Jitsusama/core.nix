# shellcheck shell=bash
# Presses mpv's keys, as its input.conf binds them, and reads back over its
# IPC socket what each did: Neovim's beside the arrows they twin, gg and
# G, y and yy, and the keys j and l displaced. mpv plays nothing to a
# screen here, so each key is pressed through mpv's own keypress command,
# which goes through the same bindings a key on the keyboard does.
# usage: keys-test.sh <config folder> <a clip of 200 seconds, a keyframe a
# second> <two subtitle files>, with a wl-copy on the config's path that
# writes what it's given to ./copied. It leaves ./passed once every key
# did what it should.
set -euo pipefail

config=$1 clip=$2 one=$3 two=$4
mpv --config-dir="$config" --vo=null --ao=null --force-window=no --pause --keep-open=yes \
  --save-position-on-quit=no --input-ipc-server="$PWD/mpv.sock" \
  --sub-file="$one" --sub-file="$two" "$clip" > said 2>&1 &
mpv=$!
trap 'kill $mpv 2>/dev/null || true' EXIT

never() {
  echo "never: $1" >&2
  cat said >&2
  exit 1
}

asked() {
  printf '{"command":%s}\n' "$1" | socat -t 2 - "UNIX-CONNECT:$PWD/mpv.sock" 2> /dev/null
}
# A property mpv has, a time in whole milliseconds.
property() {
  asked "[\"get_property\",\"$1\"]" | jq -r 'select(has("error")) | .data
    | if type == "number" then . * 1000 | round else . end'
}
key() { asked "[\"keypress\",\"$1\"]" > /dev/null; }
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

for _ in $(seq 600); do
  if [ -n "$(property duration)" ]; then break; fi
  sleep 0.1
done
[ -n "$(property duration)" ] || never "mpv to open the clip"

# From a hundred seconds in, each key lands where its arrow does, which is
# where mpv's arrows have always gone.
twins() {
  arrow=$1 letter=$2 lands=$3
  at 100
  reads time-pos 100000 "a seek to 100 seconds"
  key "$arrow"
  reads time-pos "$lands" "$arrow"
  at 100
  reads time-pos 100000 "a seek to 100 seconds"
  key "$letter"
  reads time-pos "$lands" "$letter, as $arrow"
}
twins LEFT h 95000
twins RIGHT l 105000
twins DOWN j 40000
twins UP k 160000

# gg goes to the start, as Home does, and G to the end, where keep-open
# holds the last frame.
at 100
key g
key g
reads time-pos 0 "gg"
key G
for _ in $(seq 600); do
  if [ "$(property time-pos)" -ge 198000 ]; then break; fi
  sleep 0.1
done
[ "$(property time-pos)" -ge 198000 ] || never "G to the end, not $(property time-pos)"

# J goes to the next subtitles, as j did, and L sets where a loop starts,
# as l did.
reads sid 1 "the first subtitles to be chosen"
key J
reads sid 2 "J"
at 100
reads ab-loop-a no "nothing to loop"
key L
reads ab-loop-a 100000 "L"

# y copies the frame as a picture, a PNG, and yy as y does.
copied() {
  for _ in $(seq 600); do
    if [ -s copied ] && [ "$(head -c 4 copied | tail -c 3)" = PNG ]; then return 0; fi
    sleep 0.1
  done
  never "$1 to copy the frame as a PNG"
}
key y
copied y
rm copied
key y
key y
copied yy

# f fills the screen and Escape leaves it; Space plays on and pauses.
key f
reads fullscreen true f
key ESC
reads fullscreen false Escape
key SPACE
reads pause false Space
key SPACE
reads pause true "Space again"

# q closes it.
key q
for _ in $(seq 600); do
  if ! kill -0 "$mpv" 2> /dev/null; then break; fi
  sleep 0.1
done
if kill -0 "$mpv" 2> /dev/null; then never "q to close"; fi
touch passed
