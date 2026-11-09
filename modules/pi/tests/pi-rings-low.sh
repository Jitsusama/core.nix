# shellcheck shell=bash
# Rings as the harness's done-chime-widget does, from the sound.json an
# account gets (modules/pi/chime.nix), into a file rather than speakers:
# the ding has to ring for seconds, loud enough to hear across a room.
# usage: pi-rings-low.sh <the account's sound.json>
set -euo pipefail

sound=$1

fail() {
  echo "$1"
  exit 1
}

# A figure sox's stat reads off a file, by the words its line opens with.
stat() {
  sox "$1" -n stat 2>&1 | awk -F: -v label="^$2" '$1 ~ label { gsub(/ /, "", $2); print $2 }'
}

# Whether a figure is at least another, as awk compares decimals.
at_least() {
  awk -v a="$1" -v b="$2" 'BEGIN { exit !(a >= b) }'
}

ding=$(jq -er .ding "$sound")
at_least "$(stat "$ding" Length)" 2 || fail "the ding rings for $(stat "$ding" Length) s"
at_least "$(stat "$ding" 'Maximum amplitude')" 0.8 ||
  fail "the ding peaks at $(stat "$ding" 'Maximum amplitude')"
