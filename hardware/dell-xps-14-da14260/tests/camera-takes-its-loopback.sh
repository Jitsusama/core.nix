# shellcheck shell=bash
# Runs the camera's loopback start (camera/loopback.sh), built over the
# v4l2loopback-ctl stand-in, through each start the camera service meets,
# and fails naming each that leaves the wrong loopbacks, names the wrong
# node or exits failing.
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
  echo "$2" > "$LOOPBACK_SYSFS/video$1/name"
  if [ "${3:-}" = held ]; then touch "$LOOPBACK_SYSFS/video$1/held"; fi
}
# Runs the start as the service does, keeping how it exited.
run() {
  exited=0
  "$start" "$name" "$file" -b 4 -x 1 > /dev/null || exited=$?
}
# The loopbacks there now, as node:name, lowest-numbered first, from a
# sparse array indexed by number.
there() {
  local card card_name folder
  local -a listed=()
  for card in "$LOOPBACK_SYSFS"/video*/name; do
    [ -e "$card" ] || continue
    read -r card_name < "$card" || true
    folder=${card%/name}
    listed[${folder##*/video}]="video${folder##*/video}:$card_name"
  done
  local IFS=' '
  echo "${listed[*]}"
}
expect() {
  local case=$1 nodes=$2 named=$3
  local got_nodes got_named=""
  got_nodes=$(there)
  if [ -s "$file" ]; then read -r got_named < "$file"; fi
  if [ "$exited" -ne 0 ] || [ "$got_nodes" != "$nodes" ] || [ "$got_named" != "$named" ]; then
    echo "FAIL $case: exited $exited, left [$got_nodes] naming $got_named;" \
      "meant [$nodes] naming $named"
    failures=$((failures + 1))
  else
    echo "ok $case"
  fi
}

fresh
run
expect "with none, it adds one with the options given" "video0:$name" /dev/video0
options=""
if [ -e "$LOOPBACK_SYSFS/video0/options" ]; then
  read -r options < "$LOOPBACK_SYSFS/video0/options"
fi
if [ "$options" != "-b 4 -x 1" ]; then
  echo "FAIL its options reached add as: $options"
  failures=$((failures + 1))
fi

fresh
loopback 0 "IR Camera"
loopback 1 "$name"
run
expect "with one there, it takes it and adds none" "video0:IR Camera video1:$name" /dev/video1

fresh
loopback 1 "$name" held
run
expect "with one a program still holds, it takes that one" "video1:$name" /dev/video1

fresh
loopback 4 "$name"
loopback 10 "$name"
run
expect "with two nothing holds, it keeps the lowest-numbered and deletes the other" \
  "video4:$name" /dev/video4

fresh
loopback 2 "$name" held
loopback 10 "$name"
loopback 3 "IR Camera"
run
expect "with the lowest held, it keeps that and deletes the other" \
  "video2:$name video3:IR Camera" /dev/video2

fresh
loopback 4 "$name"
loopback 5 "$name" held
run
expect "with a higher one held, it deletes the lowest and keeps the held one" \
  "video5:$name" /dev/video5

fresh
loopback 1 "$name" held
loopback 2 "$name" held
loopback 3 "$name"
run
expect "with two held, it keeps the lowest held and deletes the one nothing holds" \
  "video1:$name video2:$name" /dev/video1

fresh
loopback 0 "Built-in Camera 2"
run
expect "a loopback whose name only starts the same isn't taken" \
  "video0:Built-in Camera 2 video1:$name" /dev/video1

if [ "$failures" -ne 0 ]; then
  echo "$failures of the camera's loopback starts went wrong."
  exit 1
fi
