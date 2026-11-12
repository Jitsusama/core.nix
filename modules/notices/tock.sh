# shellcheck shell=bash
# Strikes the sound the shell makes for a message to Joel himself
# (judged.rs's chimes): a soft knock on wood, low and short, so it's
# heard beside him without carrying across the room as the want bell
# does, and is never taken for it. Two inharmonic partials, as a wooden
# block has, under 1 kHz where his dog's hearing is keenest, dying away
# in under a quarter of a second; brought to a peak well under full
# scale, since it means less than the bell.
# usage: tock <the WAV file to write>
set -euo pipefail
out=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# A partial: its frequency, how long it rings, how long it takes to die
# away, and how loud it is beside the other.
partial() {
  sox -D -n -r 48000 -c 1 -b 32 -e floating-point "$work/partial-$1.wav" \
    synth "$2" sine "$1" fade l 0.002 "$2" "$3" vol "$4"
}
partial 294 0.22 0.21 1.0
partial 812 0.09 0.085 0.3
sox -D -m "$work"/partial-*.wav -b 16 -t wav "$out" trim 0 0.22 norm -9
