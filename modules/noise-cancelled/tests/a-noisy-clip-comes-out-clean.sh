#!/usr/bin/env bash
# The noise-cancelled denoiser on a noisy clip, on the CPU: a person's
# voice over recorded noise, with a second of the noise alone before and
# after it, comes out with the noise after it at least 10 dB quieter and
# the voice within 6 dB of how it was spoken. The noise before it is only
# printed, since the denoiser is still learning the room then. On this
# room's noise the denoiser takes out about 13 dB; on steady pink noise,
# which is easier, nearly 40. The voice and the noise are the recordings
# DeepFilterNet's repository keeps.
# Run as: $BASH a-noisy-clip-comes-out-clean.sh NOISE_CANCELLED MODEL VOICE NOISE
set -euo pipefail
program=$1 model=$2 voice=$3 noise=$4

raw=(-t f32 -r 48000 -c 1)
sox "$voice" "${raw[@]}" voice.f32 remix 1 pad 1 1 norm -20
seconds=$(soxi -D "$voice")
length=$(echo "$seconds + 2" | bc)
sox "$noise" "${raw[@]}" noise.f32 remix 1 repeat 9 trim 0 "$length" norm -20
# Each at its own level, since sox would otherwise halve both to mix them.
sox -m -v 1 "${raw[@]}" voice.f32 -v 1 "${raw[@]}" noise.f32 "${raw[@]}" noisy.f32
started=$EPOCHREALTIME
"$program" clean "$model" < noisy.f32 > cleaned.f32
took=$(echo "$EPOCHREALTIME - $started" | bc)
"$program" clean "$model" < voice.f32 > alone.f32
# How fast it cleans, model loading included, since a microphone it
# can't keep up with loses what it hears.
echo "cleaned $length s of sound in $took s"

# A stretch's loudness in dB below full scale, silence's as -200 dB for bc.
loudness() {
  sox "${raw[@]}" "$1" -n trim "$2" "$3" stats 2>&1 |
    awk '/RMS lev dB/ { print ($4 == "-inf" ? -200 : $4) }'
}
# The noise alone, past the model's first frames and again past the
# voice and its last echo, and then the voice.
first=$(loudness cleaned.f32 0.2 0.7)
last=$(echo "$seconds + 1.2" | bc)
before=$(loudness noisy.f32 "$last" 0.7)
after=$(loudness cleaned.f32 "$last" 0.7)
spoken=$(loudness voice.f32 1 "$seconds")
heard=$(loudness cleaned.f32 1 "$seconds")
alone=$(loudness alone.f32 1 "$seconds")
echo "noise alone: $before dB, then $after dB, and $first dB as it started"
echo "voice: $spoken dB spoken, $heard dB through the noise, $alone dB with none"

if (($(echo "$before - $after < 10" | bc))); then
  echo "the noise is less than 10 dB quieter"
  exit 1
fi
if (($(echo "($heard - $spoken)^2 > 36" | bc))); then
  echo "the voice came out more than 6 dB from how it was spoken"
  exit 1
fi
