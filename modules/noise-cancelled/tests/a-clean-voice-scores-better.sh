#!/usr/bin/env bash
# The scorer beside Joel's ear, on the CPU: the voice alone scores a
# quieter background than the same voice over a room's noise, and better
# overall, and every mark is out of 5. The voice and the noise are the
# recordings DeepFilterNet's repository keeps, mixed as loud as each other.
# The mix is cleaned by DeepFilterNet and scored too, and its marks printed,
# so what cleaning earns is seen beside what it costs.
# DNSMOS marks the same sound differently at different levels, so each clip
# is brought to one level before it's scored, as Choose by Ear brings its
# clips to one loudness before it scores them.
# Run as: $BASH a-clean-voice-scores-better.sh NOISE_CANCELLED MODELS VOICE NOISE
set -euo pipefail
program=$1 models=$2 voice=$3 noise=$4

raw=(-t f32 -r 48000 -c 1)
# A clip's RMS amplitude, as sox measures it.
rms() { sox "${raw[@]}" "$1" -n stat 2>&1 | awk -F: '/RMS +amplitude/ { print $2 + 0 }'; }
# Brings a clip to `level` RMS, in place.
level() {
  sox "${raw[@]}" "$1" "${raw[@]}" "$1.levelled" vol "$(echo "$2 / $(rms "$1")" | bc -l)"
  mv "$1.levelled" "$1"
}

sox "$voice" "${raw[@]}" voice.f32 remix 1
seconds=$(soxi -D "$voice")
sox "$noise" "${raw[@]}" noise.f32 remix 1 repeat 9 trim 0 "$seconds"
level voice.f32 0.05
level noise.f32 0.05
sox -m -v 1 "${raw[@]}" voice.f32 -v 1 "${raw[@]}" noise.f32 "${raw[@]}" noisy.f32
"$program" clean "$models" deepfilternet <noisy.f32 >cleaned.f32

failed=0
declare -A marks
for clip in voice noisy cleaned; do
  level "$clip.f32" 0.1
  started=$EPOCHREALTIME
  marks[$clip]=$("$program" score "$models" <"$clip.f32")
  took=$(echo "$EPOCHREALTIME - $started" | bc)
  # How long it takes, its model loading included, since Choose by Ear
  # scores four clips before it reveals them.
  echo "$clip: ${marks[$clip]} in $took s"
  for mark in voice background overall; do
    said=$(jq -r ".$mark" <<<"${marks[$clip]}")
    outside=$(echo "$said < 1 || $said > 5" | bc)
    if ((outside)); then
      echo "  its $mark mark, $said, isn't out of 5"
      failed=1
    fi
  done
done

mark() { jq -r ".$2" <<<"${marks[$1]}"; }
better=$(echo "$(mark voice overall) > $(mark noisy overall)" | bc)
if ((!better)); then
  echo "the voice alone didn't score better overall than with the noise"
  failed=1
fi
alone=$(echo "$(mark voice background) - $(mark noisy background) >= 0.5" | bc)
if ((!alone)); then
  echo "the voice alone didn't score a background half a mark quieter than with the noise"
  failed=1
fi
exit "$failed"
