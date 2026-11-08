# shellcheck shell=bash
# Prints the files mpv left, for mpv's Recent in the launcher
# (MpvProgram.qml), newest first and nine at most, a line each: the file's
# whole path, where it stopped and how long it is, both in seconds, and
# when mpv left it, in seconds since 1970, a tab between each. The
# launcher says which were watched whole (Mpv.js's whole).
#
# mpv writes a file in its watch_later folder as it quits a file part way,
# named by a hash of the file's path, whose first line names the path
# (write-filename-in-watch-later-config) and whose start= says where it
# stopped; when it was written is when it was left. A redirect entry,
# which mpv writes for each folder above a file so playing the folder
# resumes it, isn't a file, nor is a stream's entry, whose name isn't a
# path, and a file that's gone can't be played, so none is listed. mpv
# keeps no length there, so ffprobe reads it from the file, and a file it
# can't read one from isn't listed: without one, how far in it stopped
# can't be said. One left in its first second wasn't watched, so it isn't
# listed either. One held on its last frame, as mpv quit at a file's end
# leaves it (keep-open), was watched whole, and is listed as it is.
#
# usage: mpv-stopped <mpv's watch_later folder>
set -euo pipefail

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
  length=$(ffprobe -v error -show_entries format=duration -of csv=p=0 -i "file:$path" \
    2> /dev/null || true)
  awk -v start="$start" -v whole="$length" \
    'BEGIN { exit !(whole + 0 > 0 && start >= 1) }' || continue
  printf '%s\t%s\t%s\t%s\n' "$path" "$start" "$length" "${left%.*}"
  listed=$((listed + 1))
  [ "$listed" -lt "$most" ] || break
done < <(find "$folder" -maxdepth 1 -type f -printf '%T@\t%p\0' | sort -z -r -n)
