# shellcheck shell=bash
# Strikes the ding pi rings when a run Joel typed is done (chime.nix says
# why it sounds as it does): a bell on G3, its partials each ringing out
# at its own pace, the higher ones sooner, as a struck bell's do, mixed
# and brought up to a peak just under full scale.
# usage: ding <the WAV file to write>
set -euo pipefail
out=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# A partial: its frequency, how long it rings, how long it takes to die
# away, and how loud it is beside the others. sox's logarithmic fade dies
# away as a bell does, fast at first and then slowly.
partial() {
  sox -D -n -r 48000 -c 1 -b 32 -e floating-point "$work/$1.wav" \
    synth "$2" sine "$1" fade l 0.003 "$2" "$3" vol "$4"
}
partial 196 6 5.99 1.0
partial 392 4 3.99 0.5
# A bell's third partial sits off the harmonics, which is what makes it
# ring rather than hum.
partial 541 2.6 2.59 0.35
partial 1058 1.2 1.19 0.12

sox -D -m "$work"/*.wav -b 16 -t wav "$out" trim 0 3 fade t 0 3 0.3 norm -1
