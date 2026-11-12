#!/usr/bin/env bash
# The scorer beside Joel's ear, on the CPU: a voice over a room's noise
# scores a quieter background once DeepFilterNet has cleaned it, and
# better overall, and every mark is out of 5. The voice alone, with no
# noise, scores a background at least as quiet as the noisy one cleaned.
# The voice and the noise are the recordings DeepFilterNet's repository
# keeps, mixed as a-noisy-clip-comes-out-clean.sh mixes them.
# Run as: $BASH a-cleaned-clip-scores-better.sh NOISE_CANCELLED MODELS VOICE NOISE
set -euo pipefail
program=$1 models=$2 voice=$3 noise=$4

raw=(-t f32 -r 48000 -c 1)
sox "$voice" "${raw[@]}" voice.f32 remix 1 norm -20
seconds=$(soxi -D "$voice")
sox "$noise" "${raw[@]}" noise.f32 remix 1 repeat 9 trim 0 "$seconds" norm -20
sox -m -v 1 "${raw[@]}" voice.f32 -v 1 "${raw[@]}" noise.f32 "${raw[@]}" noisy.f32
"$program" clean "$models" deepfilternet <noisy.f32 >cleaned.f32

failed=0
declare -A marks
for clip in voice noisy cleaned; do
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
quieter=$(echo "$(mark cleaned background) - $(mark noisy background) >= 1" | bc)
if ((!quieter)); then
  echo "cleaning the noise didn't score its background a mark quieter"
  failed=1
fi
better=$(echo "$(mark cleaned overall) > $(mark noisy overall)" | bc)
if ((!better)); then
  echo "cleaning the noise didn't score better overall"
  failed=1
fi
alone=$(echo "$(mark voice background) >= $(mark noisy background)" | bc)
if ((!alone)); then
  echo "the voice alone scored a noisier background than with the noise"
  failed=1
fi
exit "$failed"
