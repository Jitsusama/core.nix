# shellcheck shell=bash
# Strikes the sound the shell makes when a pi session first wants Joel
# (chime.nix says why it sounds as it does): the same bell on G3 as the
# done ding, struck twice in quick succession and damped sooner, so it
# reads as a call rather than an ending, brought up to a peak just under
# full scale.
# usage: want <the WAV file to write>
set -euo pipefail
out=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# A partial: its frequency, how long it rings, how long it takes to die
# away, and how loud it is beside the others, as ding.sh strikes them.
partial() {
  sox -D -n -r 48000 -c 1 -b 32 -e floating-point "$work/partial-$1.wav" \
    synth "$2" sine "$1" fade l 0.003 "$2" "$3" vol "$4"
}
partial 196 1.6 1.59 1.0
partial 392 1.2 1.19 0.5
partial 541 0.9 0.89 0.35
partial 1058 0.5 0.49 0.12
sox -D -m "$work"/partial-*.wav "$work/strike.wav"

# The second strike lands a third of a second after the first, while the
# first still rings, as a hand striking a bell twice does.
sox -D "$work/strike.wav" "$work/again.wav" pad 0.33
sox -D -m "$work/strike.wav" "$work/again.wav" -b 16 -t wav "$out" \
  trim 0 1.9 fade t 0 1.9 0.3 norm -1
