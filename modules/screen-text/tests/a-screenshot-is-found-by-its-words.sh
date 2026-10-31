#!/usr/bin/env bash
# The screen-text daemon end to end, on the CPU: a screenshot saved into
# the watched folder is read and found by its words, its lines given by
# key; removed, its record goes; and once read, a restart doesn't read it
# again. Each step waits on the daemon's own log line, never a sleep.
#
# usage: a-screenshot-is-found-by-its-words.sh SCREEN-TEXT MODELS FONT
set -euo pipefail

screen_text=$1 models=$2 font=$3
shots=$PWD/Screenshots
socket=$PWD/daemons/screen-text.sock
records=$PWD/state/records.json
export SCREEN_TEXT_DEVICE=CPU SCREEN_TEXT_CACHE=$PWD/cache
mkdir -p "$shots"

magick -size 1600x900 xc:white -font "$font" -fill black \
  -pointsize 40 -annotate +80+120 'Quarterly planning review' \
  -pointsize 24 -annotate +80+220 'Move the review to Thursday morning at ten' \
  drawn.png

# Starts the daemon, its log on descriptor 3.
start() {
  exec 3< <("$screen_text" serve "$socket" "$models" "$shots" "$records" 2>&1)
  daemon=$!
}

# Reads the daemon's log until a line holds the words given.
wait_for() {
  local line
  while IFS= read -r -t 120 line <&3; do
    echo "$line"
    if [[ $line == *"$1"* ]]; then
      return 0
    fi
  done
  echo "the daemon never said: $1"
  exit 1
}

# One ask, and its answer.
ask() {
  printf '%s\n' "$1" | socat -t 10 - "UNIX-CONNECT:$socket" | head -n 1
}

start
wait_for "answering, 0 to read"
# Moved in whole, as a picture saved elsewhere first would be.
cp drawn.png shot.png
mv shot.png "$shots/2026-10-31 09-00-00.png"
wait_for "read, 2 lines"

found=$(ask '{"id": 1, "ask": "search", "words": "THURSDAY review"}')
echo "$found"
jq -e '.records | length == 1' <<<"$found" >/dev/null
jq -e '.records[0].line == "Move the review to Thursday morning at ten"' <<<"$found" >/dev/null
jq -e '.records[0] | has("lines") | not' <<<"$found" >/dev/null
key=$(jq -r '.records[0].key' <<<"$found")

got=$(ask "$(jq -cn --arg key "$key" '{id: 2, ask: "get", keys: [$key]}')")
echo "$got"
jq -e '.records[0].lines == ["Quarterly planning review",
  "Move the review to Thursday morning at ten"]' <<<"$got" >/dev/null

none=$(ask '{"id": 3, "ask": "search", "words": "weather"}')
jq -e '.records == []' <<<"$none" >/dev/null

rm "$shots/2026-10-31 09-00-00.png"
wait_for "gone"
listed=$(ask '{"id": 4, "ask": "list"}')
jq -e '.records == []' <<<"$listed" >/dev/null

cp drawn.png "$shots/2026-10-31 10-00-00.png"
wait_for "read, 2 lines"
kill "$daemon"
wait "$daemon" || true
exec 3<&-
start
# Read before, so read again only if the record wasn't kept.
wait_for "answering, 0 to read"
listed=$(ask '{"id": 5, "ask": "list"}')
echo "$listed"
jq -e '.records | length == 1' <<<"$listed" >/dev/null
kill "$daemon"
