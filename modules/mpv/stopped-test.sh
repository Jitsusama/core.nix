# shellcheck shell=bash
# Plays clips with the account's mpv settings, each quit part way through
# or at its end with quit-watch-later as ZZ quits, and reads back what the
# launcher's lister of where mpv stopped (stopped.sh) prints of them: each
# file played on, and one held on its last frame, newest first, by its
# whole path, where it stopped, its length and when it was left; not a
# folder's redirect entry, a stream, nor a file deleted since; and nine at
# most.
# Started as Resume starts it, a file plays on from where it stopped,
# and as Play From the Start does, from its start.
# usage: stopped-test.sh <config folder> <mpv-stopped>, with mpv, ffmpeg,
# jq and socat on the path. It leaves ./passed once all of it holds.
set -euo pipefail

config=$1 stopped=$2
later=$PWD/later
mkdir -p "$later" folder
ffmpeg -loglevel error -f lavfi -i testsrc=duration=60:size=32x32:rate=2 -g 1 clip.mkv
for copy in gone.mkv end.mkv two.mkv folder/inside.mkv; do cp clip.mkv "$copy"; done

never() {
  echo "never: $1" >&2
  cat said >&2
  exit 1
}

# Before a fresh mpv makes its socket, socat goes without reading what
# printf writes, which is no matter.
asked() {
  { printf '{"command":%s}\n' "$1"; } 2> /dev/null \
    | socat -t 2 - "UNIX-CONNECT:$PWD/mpv.sock" 2> /dev/null
}
property() {
  asked "[\"get_property\",\"$1\"]" | jq -r 'select(has("error")) | .data | values'
}

# Plays what it's given from a time on, and once it's played a second past
# it, or reached its end where `end` is asked, quits it as ZZ does,
# keeping where it stopped. Up to a minute, so a busy machine can't fail it.
stop() {
  local from=$1 until=$2 what=$3
  mpv --config-dir="$config" --vo=null --ao=null --force-window=no \
    --watch-later-dir="$later" --input-ipc-server="$PWD/mpv.sock" --start="$from" \
    "$what" > said 2>&1 &
  local mpv=$!
  local played=no
  for _ in $(seq 600); do
    if [ "$until" = end ]; then
      [ "$(property eof-reached)" = true ] && played=yes && break
    else
      awk -v at="$(property time-pos)" -v from="$from" \
        'BEGIN { exit !(at != "" && at >= from + 1) }' && played=yes && break
    fi
    sleep 0.1
  done
  [ "$played" = yes ] || { kill "$mpv"; never "$what to play on from $from"; }
  asked '["quit-watch-later"]' > /dev/null
  wait "$mpv" || never "mpv to quit $what"
}

# Where each file listed was played on from.
declare -A from=([clip.mkv]=12 [folder/inside.mkv]=5 [end.mkv]=58 [two.mkv]=30)
stop 12 on clip.mkv
stop 20 on gone.mkv
rm gone.mkv
stop 5 on folder
stop 5 on 'av://lavfi:testsrc=duration=60:size=32x32:rate=2'
stop 58 end end.mkv
stop 30 on two.mkv

grep -q -x -F '# redirect entry' "$later"/* || never "playing the folder to leave a redirect entry"
grep -q -x -F "# $PWD/gone.mkv" "$later"/* || never "gone.mkv to have where it stopped kept"
grep -q -x -F "# $PWD/end.mkv" "$later"/* || never "end.mkv to have where it stopped kept"

"$stopped" "$later" > listed
cut -f 1 listed > paths
printf '%s\n' "$PWD/two.mkv" "$PWD/end.mkv" "$PWD/folder/inside.mkv" "$PWD/clip.mkv" > wanted
if ! cmp -s paths wanted; then
  echo "listed, not the files played on and to the end, newest first:" >&2
  cat listed >&2
  exit 1
fi
now=$(date +%s)
while IFS=$'\t' read -r path start length left; do
  on=${from[${path#"$PWD/"}]}
  awk -v start="$start" -v on="$on" -v whole="$length" -v left="$left" -v now="$now" \
    'BEGIN { exit !(start >= on + 1 && start < on + 30 && whole == 60 \
      && left > now - 600 && left <= now) }' \
    || never "$path to read as stopped past ${on}s of 60s, just now: $start $length $left"
  # Watched whole, as Mpv.js's whole reads it, only where it was played
  # to its end: within its last second.
  ended=no
  awk -v start="$start" -v whole="$length" 'BEGIN { exit !(start > whole - 1) }' && ended=yes
  [ "$ended" = "$([ "$path" = "$PWD/end.mkv" ] && echo yes || echo no)" ] \
    || never "$path to read as watched whole only if played to its end: $start of $length"
done < listed

# Started as Play From the Start and Resume start it (Mpv.js's play),
# paused, it starts from the start, and then from where it stopped, which
# mpv forgets once it's played on from it.
starts() {
  mpv --config-dir="$config" --vo=null --ao=null --pause --watch-later-dir="$later" \
    --save-position-on-quit=no --input-ipc-server="$PWD/mpv.sock" \
    --player-operation-mode=pseudo-gui "$@" > said 2>&1 &
  local mpv=$! at=
  for _ in $(seq 600); do
    [ "$(property seeking)" = false ] && at=$(property time-pos) && [ -n "$at" ] && break
    sleep 0.1
  done
  asked '["quit"]' > /dev/null
  wait "$mpv" || true
  echo "$at"
}
at=$(starts --no-resume-playback -- "$PWD/clip.mkv")
awk -v at="$at" 'BEGIN { exit !(at != "" && at < 1) }' \
  || never "From the Start to start at 0, not $at"
at=$(starts -- "$PWD/clip.mkv")
awk -v at="$at" 'BEGIN { exit !(at >= 13 && at < 42) }' || never "Resume to start at 13, not $at"
if grep -q -x -F "# $PWD/clip.mkv" "$later"/*; then never "mpv to forget where clip.mkv stopped"; fi

# Nine at most, the newest first: ten more left later than those, by hand
# as mpv writes them.
for n in 0 1 2 3 4 5 6 7 8 9; do
  ln -s clip.mkv "more-$n.mkv"
  printf '# %s\nstart=5.000000\n' "$PWD/more-$n.mkv" > "$later/more-$n"
  touch -d "@$((now + 10 + n))" "$later/more-$n"
done
"$stopped" "$later" | cut -f 1 > paths
for n in 9 8 7 6 5 4 3 2 1; do echo "$PWD/more-$n.mkv"; done > wanted
cmp -s paths wanted || { echo "listed, not the nine newest:" >&2; cat paths >&2; exit 1; }

touch passed
