#!/usr/bin/env bash
# The meaning daemon's pictures end to end, on the CPU, with the SigLIP 2
# towers the service runs: given a folder of pictures, it looks at each,
# and finds each by words saying what it shows, and only it; words
# showing none of them find nothing; a picture saved while it runs is
# heard and found; and once their looks are kept, a restart looks at
# nothing again. The pictures are drawn here, so nothing in them is a
# name or a word to find. Each step waits on the daemon's own log line or
# answer, never a sleep.
#
# usage: a-picture-is-found-by-what-it-shows.sh MEANING MODELS SIGHT
set -euo pipefail

meaning=$1 models=$2 sight=$3
socket=$PWD/daemons/meaning.sock
shots=$PWD/Pictures/Screenshots
export MEANING_DEVICE=CPU MEANING_CACHE=$PWD/cache
mkdir -p "$PWD/daemons" "$PWD/state" "$shots"

magick -size 224x224 xc:white -fill red -draw "circle 112,112 112,40" "$shots/one.png"
magick -size 224x224 xc:white -fill blue -draw "rectangle 40,40 184,184" "$shots/two.png"

# Starts the daemon, its log on descriptor 3.
start() {
  exec 3< <("$meaning" serve "$socket" "$models" "$PWD/state/meanings" \
    "$sight" "$PWD/state/looks" "$shots" "$PWD/Pictures/Camera" 2>&1)
  daemon=$!
}

# Reads the daemon's log until a line holds the words given.
wait_for() {
  local line
  while IFS= read -r -t 300 line <&3; do
    echo "$line"
    if [[ $line == *"$1"* ]]; then
      return 0
    fi
  done
  echo "the daemon never said: $1"
  exit 1
}

# A search of the pictures, answered once its words are embedded.
search() {
  jq -cn --arg words "$1" '{id: 1, ask: "search", kind: "picture", words: $words}' |
    socat -t 300 - "UNIX-CONNECT:$socket" | head -n 1
}

# Words find the one picture showing what they say, and no other.
finds() {
  local found
  found=$(search "$1")
  echo "$1: $found"
  jq -e --arg want "$shots/$2" '.whole and (.records | map(.key)) == [$want]
    and .records[0].kind == "picture"' <<<"$found" >/dev/null
}

start
wait_for "answering, 0 meanings kept, 2 pictures to look at"
wait_for "2 pictures looked at, 2 kept"
finds "a red circle" one.png
finds "a blue square" two.png
none=$(search "a cat")
echo "nothing: $none"
jq -e '.whole and .records == []' <<<"$none" >/dev/null

# One saved while it runs is heard, looked at and found.
magick -size 224x224 xc:white -fill green -draw "polygon 112,30 200,190 24,190" \
  "$shots/three.png"
wait_for "1 pictures looked at, 3 kept"
finds "a green triangle" three.png

kill "$daemon"
wait "$daemon" || true
exec 3<&-

# Kept: a restart has nothing to look at, and finds at once.
start
wait_for "0 pictures to look at"
finds "a red circle" one.png
kill "$daemon"
