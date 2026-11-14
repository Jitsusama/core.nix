# shellcheck shell=bash
# Prints the files mpv left, for mpv's Recent in the launcher
# (MpvProgram.qml), newest first and nine at most, a line each: the file's
# whole path, where it stopped and how long it is, both in seconds, when
# mpv left it, in seconds since 1970, its size in bytes, whether it's a
# video or audio alone, and then each other option mpv kept for it, as
# mpv wrote it (speed=1.500000), a tab between each. The launcher says
# which were watched whole (Mpv.js's whole), and its drawer says what mpv
# kept, so its speed, volume and tracks are seen before it plays on.
#
# mpv writes a file in its watch_later folder as it quits a file part way,
# named by a hash of the file's path, whose first line names the path
# (write-filename-in-watch-later-config) and whose start= says where it
# stopped; when it was written is when it was left. mpv writes there only
# the options changed while it played, so each is passed on as it is,
# but for one holding a tab, which would break its line. A redirect entry,
# which mpv writes for each folder above a file so playing the folder
# resumes it, isn't a file, nor is a stream's entry, whose name isn't a
# path, and a file that's gone can't be played, so none is listed. mpv
# keeps no length there, so ffprobe reads it from the file, with its
# streams, and a file it can't read one from isn't listed: without one,
# how far in it stopped can't be said. A file is a video when it holds a
# picture that moves, so a song with its cover is audio. One left in its
# first second wasn't watched, so it isn't listed either. One held on its
# last frame, as mpv quit at a file's end leaves it (keep-open), was
# watched whole, and is listed as it is.
#
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.
set -euo pipefail

case "${1:-}" in
-h | --help)
  cat << 'EOF'
mpv-stopped  list the files mpv stopped part way through

  For mpv's Recent in the launcher: newest first and nine
  at most, a line each of the file, where it stopped, its
  length, when it was left, its size, whether it's video
  or audio, and each option mpv kept, a tab between each.

EXAMPLES
  mpv-stopped ~/.local/state/mpv/watch_later

EXIT
  0   listed, or nothing to list
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -ne 1 ]; then
  printf '%s\n' "mpv-stopped takes mpv's watch_later folder" \
    "  try  mpv-stopped --help" >&2
  exit 64
fi

folder=$1
most=9
listed=0
[ -d "$folder" ] || exit 0

while IFS=$'\t' read -r -d '' left kept; do
  first=
  IFS= read -r first < "$kept" || [ -n "$first" ] || continue
  case $first in
    '# /'*) path=${first#'# '} ;;
    *) continue ;;
  esac
  [ -f "$path" ] || continue
  start=$(sed -n '/^start=/{s///p;q}' "$kept")
  [ -n "$start" ] || continue
  # Each stream's line names its section first (stream,video,0, its last
  # figure whether it's a cover), and the format's (format,60.000000).
  probed=$(ffprobe -v error -of csv \
    -show_entries format=duration:stream=codec_type:stream_disposition=attached_pic \
    -i "file:$path" 2> /dev/null || true)
  length=$(awk -F, '$1 == "format" { print $2; exit }' <<< "$probed")
  kind=$(awk -F, '$1 == "stream" && $2 == "video" && $3 != 1 { moves = 1 }
    END { print moves ? "video" : "audio" }' <<< "$probed")
  awk -v start="$start" -v whole="$length" \
    'BEGIN { exit !(whole + 0 > 0 && start >= 1) }' || continue
  size=$(stat -L -c %s -- "$path")
  options=$(awk '/^[a-z][a-z0-9-]*=/ && !/^start=/ && !/\t/ { printf "\t%s", $0 }' "$kept")
  printf '%s\t%s\t%s\t%s\t%s\t%s%s\n' \
    "$path" "$start" "$length" "${left%.*}" "$size" "$kind" "$options"
  listed=$((listed + 1))
  [ "$listed" -lt "$most" ] || break
done < <(find "$folder" -maxdepth 1 -type f -printf '%T@\t%p\0' | sort -z -r -n)
