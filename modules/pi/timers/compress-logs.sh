# shellcheck shell=bash
# Rewrites each closed pi session log in the folder it's given compressed
# on disk, one at a time, waiting while other work waits on the disk.
# btrfs compresses as it writes, but a log written in small appends is
# mostly stored as it came; rewriting it compressed changes nothing a
# reader sees, as the file's bytes and modification time stay the same.
# On anything but btrfs there is nothing to do. Its help takes the shape
# every command of ours does (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
pi-compress-logs  keep closed pi session logs compressed

  Rewrites each log in the sessions folder untouched for
  two days and past 64 KiB compressed on disk, one at a
  time, waiting while other work waits on the disk. What
  a reader sees stays the same. Off btrfs it does nothing.
  A timer runs it each night.

EXAMPLES
  pi-compress-logs ~/.pi/agent/sessions

EXIT
  0   rewritten, or nothing to do
  64  asked wrongly
EOF
  exit 0
  ;;
"")
  echo "pi-compress-logs: name the sessions folder; --help says more" >&2
  exit 64
  ;;
*) ;;
esac
sessions=$1
# A log under 64 KiB gains little, and one touched in the last two days
# may still be growing.
smallest=64k
untouched_days=1
# A log whose extents are already this share compressed is left alone, so
# a nightly run reads only what's new.
compressed_enough=90
# While I/O pressure's full share over ten seconds is at or past this,
# other work is waiting on the disk, and the run waits for it.
busy_disk=12

file_system=$(stat -f -c %T "$sessions")
if [ "$file_system" != btrfs ]; then
  echo "$sessions isn't on btrfs, so there's nothing to compress"
  exit 0
fi

# The bytes used on the sessions folder's file system, the last line of
# df's answer.
used() {
  local report
  report=$(df -B1 --output=used "$sessions")
  echo "${report##*[!0-9]}"
}

disk_waits() {
  awk -v busy="$busy_disk" '/^full/ { split($2, avg, "="); exit !(avg[2] + 0 >= busy) }' /proc/pressure/io
}

# The logs to look at, found once before any is rewritten, so the walk
# never meets a file it has just written.
logs=$(mktemp)
trap 'rm -f "$logs"' EXIT
find "$sessions" -name '*.jsonl' -mtime +"$untouched_days" -size +"$smallest" -print0 > "$logs"

before=$(used)
rewritten=0
while IFS= read -r -d '' log; do
  # filefrag -v lists each extent, its length fourth, and "encoded"
  # among the flags of one stored compressed. A log gone since find saw
  # it is skipped.
  extents=$(filefrag -v "$log" 2> /dev/null) || continue
  shares=$(awk -F: '/^ *[0-9]+:/ { length_ = $4 + 0; all += length_; if ($0 ~ /encoded/) packed += length_ }
    END { print all + 0, packed + 0 }' <<< "$extents")
  read -r all packed <<< "$shares"
  if [ "$all" -gt 0 ] && [ $((100 * packed / all)) -ge "$compressed_enough" ]; then
    continue
  fi
  while disk_waits; do
    sleep 5
  done
  if btrfs filesystem defragment -czstd "$log"; then
    # Written out now, a log at a time, rather than all at once later.
    sync -f "$log"
    rewritten=$((rewritten + 1))
  else
    echo "couldn't rewrite $log compressed; it stays as it was"
  fi
done < "$logs"
after=$(used)
echo "rewrote $rewritten logs compressed; $sessions's file system went from $before to $after bytes used"
