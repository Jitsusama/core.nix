# shellcheck shell=bash
# Rings as the harness's done-chime-widget does, from the sound.json an
# account gets (modules/pi/chime.nix), into files rather than speakers:
# piper speaks a line with the account's voice and sox lowers it by the
# account's pitch. The ding has to ring for seconds, loud; the line has
# to be speech, near full scale; and lowered, it has to sit lower than
# piper said it. A voice piper can't load is silence at a ring, never an
# error, so this is where it would show.
# usage: pi-rings-low-and-says-it.sh <the account's sound.json>
set -euo pipefail

sound=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

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

field() {
  jq -er ".$1" "$sound"
}

ding=$(field ding)
at_least "$(stat "$ding" Length)" 2 || fail "the ding rings for $(stat "$ding" Length) s"
at_least "$(stat "$ding" 'Maximum amplitude')" 0.8 ||
  fail "the ding peaks at $(stat "$ding" 'Maximum amplitude')"

"$(field piper)" --model "$(field voice)" --output-file "$work/said.wav" \
  -- "Agentic loop is ready." || fail "piper can't speak with the account's voice"
"$(field sox)" "$work/said.wav" "$work/low.wav" pitch "$(field pitch)"

at_least "$(stat "$work/low.wav" Length)" 1 || fail "the line lasts $(stat "$work/low.wav" Length) s"
at_least "$(stat "$work/low.wav" 'Maximum amplitude')" 0.8 ||
  fail "the line peaks at $(stat "$work/low.wav" 'Maximum amplitude')"
# sox's rough frequency counts the line's crossings of zero, which a line
# moved lower crosses less often.
said=$(stat "$work/said.wav" Rough)
low=$(stat "$work/low.wav" Rough)
if at_least "$low" "$said"; then
  fail "lowered, the line reads $low Hz to piper's $said"
fi
