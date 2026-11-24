# Rewrites each closed pi session log in the folder it's given compressed
# on disk, one at a time, waiting while other work waits on the disk.
# btrfs compresses as it writes, but a log written in small appends is
# mostly stored as it came; rewriting it compressed changes nothing a
# reader sees, as the file's bytes and modification time stay the same.
# On anything but btrfs there is nothing to do.
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

if [ "$(stat -f -c %T "$sessions")" != btrfs ]; then
  echo "$sessions isn't on btrfs, so there's nothing to compress"
  exit 0
fi

used() {
  df -B1 --output=used "$sessions" | tail -1
}

disk_waits() {
  awk -v busy="$busy_disk" '/^full/ { split($2, avg, "="); exit !(avg[2] + 0 >= busy) }' /proc/pressure/io
}

before=$(used)
rewritten=0
while IFS= read -r -d '' log; do
  # filefrag -v lists each extent, its length fourth, and "encoded"
  # among the flags of one stored compressed. A log gone since find saw
  # it is skipped.
  shares=$(filefrag -v "$log" 2>/dev/null |
    awk -F: '/^ *[0-9]+:/ { length_ = $4 + 0; all += length_; if ($0 ~ /encoded/) packed += length_ }
      END { print all + 0, packed + 0 }') || continue
  read -r all packed <<<"$shares"
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
done < <(find "$sessions" -name '*.jsonl' -mtime +"$untouched_days" -size +"$smallest" -print0)
echo "rewrote $rewritten logs compressed; $sessions's file system went from $before to $(used) bytes used"
