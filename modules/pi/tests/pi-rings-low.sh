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

# Reads sox's stat of a sound from its input and fails, saying what fell
# short, unless the sound rings for at least the seconds given, peaks at
# 0.8 or more, and sits under 500 Hz.
rings_as_heard() {
  awk -F: -v what="$1" -v long="$2" '
    { gsub(/ /, "", $2) }
    $1 ~ /^Length/ { seconds = $2 + 0 }
    $1 ~ /^Maximum amplitude/ { peak = $2 + 0 }
    $1 ~ /^Rough/ { pitch = $2 + 0 }
    END {
      if (seconds < long) { print "the " what " rings for " seconds " s"; exit 1 }
      if (peak < 0.8) { print "the " what " peaks at " peak; exit 1 }
      if (pitch == 0 || pitch >= 500) { print "the " what " sits at " pitch " Hz"; exit 1 }
    }'
}

ding=$(jq -er .ding "${sound}")
sox "${ding}" -n stat 2>&1 | rings_as_heard ding 2
sox "${want}" -n stat 2>&1 | rings_as_heard want 1.5
