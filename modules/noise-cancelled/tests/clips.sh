#!/usr/bin/env bash
# What noise-cancelled's checks share for handling a clip: raw 48 kHz mono
# floats, as the program reads and writes them, brought to one level before
# they're scored, since DNSMOS marks the same sound differently at each
# level. Sourced, after the script sets `program` and `models`.

raw=(-t f32 -r 48000 -c 1)

# A clip's RMS amplitude, as sox measures it.
rms() {
  local stats
  stats=$(sox "${raw[@]}" "$1" -n stat 2>&1)
  awk -F: '/RMS +amplitude/ { print $2 + 0 }' <<<"$stats"
}

# Brings a clip to `level` RMS, in place.
level() {
  local now gain
  now=$(rms "$1")
  gain=$(echo "$2 / $now" | bc -l)
  sox "${raw[@]}" "$1" "${raw[@]}" "$1.levelled" vol "$gain"
  mv "$1.levelled" "$1"
}

# A clip's marks from the scorer, as JSON.
score() { "${program:?}" score "${models:?}" <"$1"; }
