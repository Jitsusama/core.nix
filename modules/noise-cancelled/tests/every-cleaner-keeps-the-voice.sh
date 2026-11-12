#!/usr/bin/env bash
# Every cleaner, as the DNSMOS scorer hears it, makes a voice in noise
# sound better overall and leaves the voice itself almost as it was: its
# voice mark falls no more than 0.3 under the noisy one's. The voice is the
# recording DeepFilterNet's repository keeps, in two noises as loud as it:
# the room that repository keeps, and pink noise, steady from end to end.
# Each clip is levelled alike before it's scored, as Choose by Ear levels
# its clips, since DNSMOS marks the same sound differently at each level.
# Run as: $BASH every-cleaner-keeps-the-voice.sh NOISE_CANCELLED MODELS VOICE NOISE
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
score() { "$program" score "$models" <"$1"; }
mark() { jq -r ".$2" <<<"$1"; }

sox "$voice" "${raw[@]}" voice.f32 remix 1
seconds=$(soxi -D "$voice")
sox "$noise" "${raw[@]}" room.f32 remix 1 repeat 9 trim 0 "$seconds"
# Repeatable, so the same pink noise is heard on every run.
sox -R -n "${raw[@]}" pink.f32 synth "$seconds" pinknoise
level voice.f32 0.05

failed=0
for noise in room pink; do
  level "$noise.f32" 0.05
  sox -m -v 1 "${raw[@]}" voice.f32 -v 1 "${raw[@]}" "$noise.f32" "${raw[@]}" "$noise-noisy.f32"
  for cleaner in deepfilternet deepfilternet-gentle rnnoise; do
    "$program" clean "$models" "$cleaner" <"$noise-noisy.f32" >"$noise-$cleaner.f32"
  done
  cp "$noise-noisy.f32" "$noise-heard.f32"
  level "$noise-heard.f32" 0.1
  noisy=$(score "$noise-heard.f32")
  echo "$noise, noisy: $noisy"
  for cleaner in deepfilternet deepfilternet-gentle rnnoise; do
    level "$noise-$cleaner.f32" 0.1
    cleaned=$(score "$noise-$cleaner.f32")
    echo "$noise, $cleaner: $cleaned"
    better=$(echo "$(mark "$cleaned" overall) > $(mark "$noisy" overall)" | bc)
    if ((!better)); then
      echo "  $cleaner made the voice in $noise noise sound no better overall"
      failed=1
    fi
    kept=$(echo "$(mark "$cleaned" voice) >= $(mark "$noisy" voice) - 0.3" | bc)
    if ((!kept)); then
      echo "  $cleaner hurt the voice itself in $noise noise"
      failed=1
    fi
  done
done
exit "$failed"
