#!/usr/bin/env bash
# The meaning daemon end to end, on the CPU, with the model the service
# runs: told the launcher's things, it finds each by words that share none
# of its letters, nearest first; words that mean none of them find
# nothing; and once their meanings are kept, a restart answers whole at
# once rather than embedding them again. Each step waits on the daemon's
# own log line or answer, never a sleep.
#
# usage: a-thing-is-found-by-its-meaning.sh MEANING MODELS
set -euo pipefail

meaning=$1 models=$2
socket=$PWD/daemons/meaning.sock
meanings=$PWD/state/meanings
export MEANING_DEVICE=CPU MEANING_CACHE=$PWD/cache
mkdir -p "$PWD/daemons" "$PWD/state"

# The things as the launcher tells them: a name and the words it's known by.
things='[
  {"key": "action:brightness", "text": "Brightness screen backlight"},
  {"key": "action:volume", "text": "Volume sound speakers loudness"},
  {"key": "action:wifi", "text": "Wi-Fi wifi wireless network"},
  {"key": "action:power-off", "text": "Power Off shut down"},
  {"key": "action:lock", "text": "Lock the Screen"},
  {"key": "program:calculator", "text": "Calculator maths sums"},
  {"key": "program:mail", "text": "Mail email inbox"},
  {"key": "collection:emoji", "text": "Emoji smileys"}
]'
count=$(jq length <<<"$things")

# Starts the daemon, its log on descriptor 3.
start() {
  exec 3< <("$meaning" serve "$socket" "$models" "$meanings" 2>&1)
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

# One ask, and its answer, which a search gives once its words are embedded.
ask() {
  printf '%s\n' "$1" | socat -t 300 - "UNIX-CONNECT:$socket" | head -n 1
}

search() {
  ask "$(jq -cn --arg words "$1" '{id: 1, ask: "search", kind: "thing", words: $words}')"
}

start
wait_for "answering, 0 meanings kept"
told=$(ask "$(jq -cn --argjson things "$things" '{id: 1, ask: "know", kind: "thing",
  things: $things}')")
echo "$told"
jq -e --argjson count "$count" '.known == $count' <<<"$told" >/dev/null
wait_for "$count things meant, $count kept"

# Each found by what it means, its letters nowhere in what's typed.
for pair in "dim the display:action:brightness" "turn the computer off:action:power-off" \
  "make it louder:action:volume" "add up some numbers:program:calculator"; do
  words=${pair%%:*} want=${pair#*:}
  found=$(search "$words")
  echo "$words: $found"
  jq -e --arg want "$want" '.whole and .records[0].key == $want' <<<"$found" >/dev/null
  jq -e '.records | map(.near) | . == (sort | reverse)' <<<"$found" >/dev/null
done

# Words meaning nothing it holds find nothing, rather than its nearest.
none=$(search "a recipe for banana bread")
echo "nothing: $none"
jq -e '.whole and .records == []' <<<"$none" >/dev/null

kill "$daemon"
wait "$daemon" || true
exec 3<&-

# Kept: a restart holds every meaning, so told again it answers whole.
start
wait_for "answering, $count meanings kept"
ask "$(jq -cn --argjson things "$things" '{id: 1, ask: "know", kind: "thing",
  things: $things}')" >/dev/null
again=$(search "dim the display")
echo "again: $again"
jq -e '.whole and .records[0].key == "action:brightness"' <<<"$again" >/dev/null
kill "$daemon"
