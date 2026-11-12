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
# Run as: $BASH a-clean-voice-scores-better.sh CLIPS NOISE_CANCELLED MODELS VOICE NOISE
set -euo pipefail
clips=$1 program=$2 models=$3 voice=$4 noise=$5
# The library is handed in by its store path; shellcheck reads it from here.
# shellcheck source=SCRIPTDIR/clips.sh
source "$clips"

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
  marks[$clip]=$(score "$clip.f32")
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
voice_overall=$(mark voice overall)
noisy_overall=$(mark noisy overall)
voice_background=$(mark voice background)
noisy_background=$(mark noisy background)
better=$(echo "$voice_overall > $noisy_overall" | bc)
if ((!better)); then
  echo "the voice alone didn't score better overall than with the noise"
  failed=1
fi
alone=$(echo "$voice_background - $noisy_background >= 0.5" | bc)
if ((!alone)); then
  echo "the voice alone didn't score a background half a mark quieter than with the noise"
  failed=1
fi
exit "$failed"
