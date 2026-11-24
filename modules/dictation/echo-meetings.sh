# shellcheck shell=bash
# Lays the meetings out again with the far end mixed into Joel's side 50 ms
# late and 20 dB down, as his microphone hears his speakers on a call with
# no echo cancelling; the far end's side and the references are copied
# as they are (echoed-meetings.nix).
# usage: echo-meetings.sh <the meetings' folder> <where they go>
set -euo pipefail

meetings=$1
out=$2
mkdir "$out"
cp "$meetings"/sides "$meetings"/*.ref "$meetings"/*.theirs.wav "$out"/
while read -r name _; do
  [ -n "$name" ] || continue
  ffmpeg -nostdin -loglevel error -i "$meetings/$name.mine.wav" -i "$meetings/$name.theirs.wav" \
    -filter_complex "[1]adelay=50,volume=-20dB[e];[0][e]amix=inputs=2:duration=first:normalize=0" \
    -ar 16000 -ac 1 -c:a pcm_s16le "$out/$name.mine.wav"
done <"$meetings"/sides
