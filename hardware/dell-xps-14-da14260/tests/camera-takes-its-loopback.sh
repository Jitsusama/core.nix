# shellcheck shell=bash
# Runs the camera's loopback start (camera/loopback.sh), built over the
# v4l2loopback-ctl stand-in, through each start the camera service meets,
# and fails naming the first that leaves the wrong loopbacks.
#
# Usage: camera-takes-its-loopback.sh <the start, built over the stand-in>
set -euo pipefail

start=$1
name="Built-in Camera"
export LOOPBACK_SYSFS
failures=0

# A fresh folder of loopbacks, and the file the start names its node in.
fresh() {
  LOOPBACK_SYSFS=$(mktemp -d)
  file=$(mktemp)
}
# Makes a loopback as the module would, held when asked.
loopback() {
  mkdir "$LOOPBACK_SYSFS/video$1"
  echo "$2" >"$LOOPBACK_SYSFS/video$1/name"
  if [ "${3:-}" = held ]; then touch "$LOOPBACK_SYSFS/video$1/held"; fi
}
# The loopbacks there now, as node:name, in order.
there() {
  for card in "$LOOPBACK_SYSFS"/video*/name; do
    [ -e "$card" ] && echo "$(basename "$(dirname "$card")"):$(cat "$card")"
  done | sort -V | paste -sd ' '
}
expect() {
  local case=$1 nodes=$2 named=$3
  local got_nodes got_named
  got_nodes=$(there)
  got_named=$(cat "$file")
  if [ "$got_nodes" != "$nodes" ] || [ "$got_named" != "$named" ]; then
    echo "FAIL $case: left [$got_nodes] naming $got_named; meant [$nodes] naming $named"
    failures=$((failures + 1))
  else
    echo "ok $case"
  fi
}

fresh
"$start" "$name" "$file" -b 4 -x 1 >/dev/null
expect "with none, it adds one with the options given" "video0:$name" /dev/video0
[ "$(cat "$LOOPBACK_SYSFS/video0/options")" = "-b 4 -x 1" ] || {
  echo "FAIL its options reached add as: $(cat "$LOOPBACK_SYSFS/video0/options")"
  failures=$((failures + 1))
}

fresh
loopback 0 "IR Camera"
loopback 1 "$name"
"$start" "$name" "$file" -b 4 -x 1 >/dev/null
expect "with one there, it takes it and adds none" "video0:IR Camera video1:$name" /dev/video1

fresh
loopback 1 "$name" held
"$start" "$name" "$file" -b 4 -x 1 >/dev/null
expect "with one a program still holds, it takes that one" "video1:$name" /dev/video1

fresh
loopback 2 "$name" held
loopback 10 "$name"
loopback 3 "IR Camera"
"$start" "$name" "$file" -b 4 -x 1 >/dev/null
expect "with one left from before, it keeps the oldest and deletes the other" \
  "video2:$name video3:IR Camera" /dev/video2

fresh
loopback 4 "$name"
loopback 5 "$name" held
"$start" "$name" "$file" -b 4 -x 1 >/dev/null
expect "with the one left from before held, it keeps the oldest and leaves that" \
  "video4:$name video5:$name" /dev/video4

fresh
loopback 0 "Built-in Camera 2"
"$start" "$name" "$file" -b 4 -x 1 >/dev/null
expect "a loopback whose name only starts the same isn't taken" \
  "video0:Built-in Camera 2 video1:$name" /dev/video1

if [ "$failures" -ne 0 ]; then
  echo "$failures of the camera's loopback starts went wrong."
  exit 1
fi
