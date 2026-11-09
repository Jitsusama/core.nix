# shellcheck shell=bash
# Rings as the harness's done-chime-widget does, from the sound.json an
# account gets (modules/pi/chime.nix), into a file rather than speakers:
# the ding has to ring for seconds, loud enough to hear across a room.
# The shell's want sound has to ring loud too, if for less long. Both sit
# low, under the 700 Hz sox reads off the theme's message, the sound that
# scared Joel's dog.
# usage: pi-rings-low.sh <the account's sound.json> <the shell's want sound>
set -euo pipefail

sound=$1
want=$2

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
at_least 500 "$(stat "$ding" 'Rough')" || fail "the ding sits at $(stat "$ding" 'Rough') Hz"

at_least "$(stat "$want" Length)" 1.5 || fail "the want rings for $(stat "$want" Length) s"
at_least "$(stat "$want" 'Maximum amplitude')" 0.8 ||
  fail "the want peaks at $(stat "$want" 'Maximum amplitude')"
at_least 500 "$(stat "$want" 'Rough')" || fail "the want sits at $(stat "$want" 'Rough') Hz"
