#!/usr/bin/env bash
# Each of the noise-cancelled cleaners on a noisy clip, on the CPU: a
# person's voice over recorded noise, with a second of the noise alone
# before and after it, comes out with the noise after it quieter by at
# least as much as the cleaner promises, and the voice within 6 dB of how
# it was spoken. The gentle cleaner keeps more of the room than the full
# one, since keeping it is what it's for. The noise before the voice is
# only printed, since a cleaner is still learning the room then. The voice
# and the noise are the recordings DeepFilterNet's repository keeps.
# Run as: $BASH a-noisy-clip-comes-out-clean.sh NOISE_CANCELLED MODEL VOICE NOISE
set -euo pipefail
program=$1 model=$2 voice=$3 noise=$4

# How much quieter each cleaner takes this room's noise at the least. On
# it, on 11 November, DeepFilterNet took out 13 dB, its gentle one 8 and
# RNNoise 16; on steady pink noise, which is easier, DeepFilterNet takes
# out nearly 40.
declare -A promised=(
  [deepfilternet]=10
  [deepfilternet-gentle]=6
  [rnnoise]=6
)

raw=(-t f32 -r 48000 -c 1)
sox "$voice" "${raw[@]}" voice.f32 remix 1 pad 1 1 norm -20
seconds=$(soxi -D "$voice")
length=$(echo "$seconds + 2" | bc)
sox "$noise" "${raw[@]}" noise.f32 remix 1 repeat 9 trim 0 "$length" norm -20
# Each at its own level, since sox would otherwise halve both to mix them.
sox -m -v 1 "${raw[@]}" voice.f32 -v 1 "${raw[@]}" noise.f32 "${raw[@]}" noisy.f32

# A stretch's loudness in dB below full scale, silence's as -200 dB for bc.
loudness() {
  sox "${raw[@]}" "$1" -n trim "$2" "$3" stats 2>&1 |
    awk '/RMS lev dB/ { print ($4 == "-inf" ? -200 : $4) }'
}
last=$(echo "$seconds + 1.2" | bc)
before=$(loudness noisy.f32 "$last" 0.7)
spoken=$(loudness voice.f32 1 "$seconds")
echo "noise alone: $before dB; voice: $spoken dB spoken"

failed=0
declare -A left
for cleaner in deepfilternet deepfilternet-gentle rnnoise; do
  started=$EPOCHREALTIME
  "$program" clean "$model" "$cleaner" <noisy.f32 >"$cleaner.f32"
  took=$(echo "$EPOCHREALTIME - $started" | bc)
  "$program" clean "$model" "$cleaner" <voice.f32 >"$cleaner-alone.f32"
  # The noise alone, past the cleaner's first frames and again past the
  # voice and its last echo, and then the voice.
  first=$(loudness "$cleaner.f32" 0.2 0.7)
  after=$(loudness "$cleaner.f32" "$last" 0.7)
  heard=$(loudness "$cleaner.f32" 1 "$seconds")
  alone=$(loudness "$cleaner-alone.f32" 1 "$seconds")
  left[$cleaner]=$after
  # How fast it cleans, its model loading included, since a microphone
  # it can't keep up with loses what it hears.
  echo "$cleaner: cleaned $length s of sound in $took s"
  echo "  noise $after dB, and $first dB as it started"
  echo "  voice $heard dB through the noise, $alone dB with none"
  short=$(echo "$before - $after < ${promised[$cleaner]}" | bc)
  if ((short)); then
    echo "  the noise is less than ${promised[$cleaner]} dB quieter"
    failed=1
  fi
  moved=$(echo "($heard - $spoken)^2 > 36" | bc)
  if ((moved)); then
    echo "  the voice came out more than 6 dB from how it was spoken"
    failed=1
  fi
done

harsher=$(echo "${left[deepfilternet-gentle]} <= ${left[deepfilternet]}" | bc)
if ((harsher)); then
  echo "the gentle cleaner kept no more of the room than the full one"
  failed=1
fi
exit "$failed"
