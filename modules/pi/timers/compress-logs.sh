# shellcheck shell=bash
# Rewrites each closed pi session log in the folder it's given compressed
# on disk, one at a time, waiting while other work waits on the disk.
# btrfs compresses as it writes, but a log written in small appends is
# mostly stored as it came; rewriting it compressed changes nothing a
# reader sees, as the file's bytes and modification time stay the same.
# It checks that of each log it rewrites by reading it back once the kernel
# has dropped the copy it held in memory, so the read comes from the
# compressed extents on disk. A log a session writes to during its rewrite
# is passed over, as one reopened before it is. On anything but btrfs there
# is nothing to do. Its help takes the shape every command of ours does
# (docs/commands.md), by hand.

say_help() {
  cat << 'EOF'
pi-compress-logs  keep closed pi session logs compressed

  Rewrites each log in the sessions folder untouched for
  two days and past 64 KiB compressed on disk, one at a
  time, waiting while other work waits on the disk. It
  reads each log back from disk after its rewrite and
  checks it reads the same; one a session writes to
  meanwhile is left for a night it's closed. Off btrfs
  it does nothing. A timer runs it each night.

EXAMPLES
  pi-compress-logs ~/.pi/agent/sessions

EXIT
  0   rewritten, or nothing to do
  1   a log couldn't be rewritten, or read otherwise after
  64  asked wrongly
  66  no such sessions folder
EOF
}

for word in "$@"; do
  case $word in
  -h | --help)
    say_help
    exit 0
    ;;
  *) ;;
  esac
done
case "$#:${1:-}" in
1:-*)
  echo "pi-compress-logs: $1 isn't an option it takes; --help says more" >&2
  exit 64
  ;;
1:?*) ;;
*)
  echo "pi-compress-logs: name the sessions folder, and only it; --help says more" >&2
  exit 64
  ;;
esac
sessions=$1
if [ ! -d "$sessions" ]; then
  echo "pi-compress-logs: there's no folder at $sessions" >&2
  exit 66
fi
# A log under 64 KiB gains little, and one touched in the last two days
# may still be growing: find's -mtime +1 takes those two whole days or
# more past their last write.
smallest=64k
whole_days_past=1
# A log whose extents are already this share compressed is left alone, so
# a nightly run reads only what's new.
compressed_enough=90
# While I/O pressure's full share over ten seconds is at or past this,
# other work is waiting on the disk, and the run waits for it.
busy_disk=12
# Where the kernel says how long work waits on the disk. A check names a
# file of its own here, to stand in for the kernel.
pressure=${PI_COMPRESS_LOGS_PRESSURE:-/proc/pressure/io}

file_system=$(stat -f -c %T -- "$sessions")
if [ "$file_system" != btrfs ]; then
  echo "$sessions isn't on btrfs, so there's nothing to compress"
  exit 0
fi

# The bytes used on the sessions folder's file system, the last line of
# df's answer.
used() {
  local report
  report=$(df -B1 --output=used -- "$sessions")
  echo "${report##*[!0-9]}"
}

# Whether other work waits on the disk. A kernel that doesn't say, with no
# pressure file to read, is taken as never waiting.
disk_waits() {
  [ -r "$pressure" ] || return 1
  awk -v busy="$busy_disk" '/^full/ { split($2, avg, "="); exit !(avg[2] + 0 >= busy) }' "$pressure"
}

# Whether the log at $1 is still untouched long enough to be closed.
still_closed() {
  local found
  found=$(find "$1" -maxdepth 0 -mtime +"$whole_days_past" -print 2> /dev/null) || return 1
  [ -n "$found" ]
}

# What a reader sees of the log at $1: its length, its modification time
# and its bytes' checksum.
seen() {
  local said
  local sum
  said=$(stat -c '%s %Y' -- "$1") || return 1
  sum=$(cksum < "$1") || return 1
  echo "$said $sum"
}

# Whether a session wrote to the log between the two readings $1 and $2 of
# it, as seen gives them: it grew and its modification time moved on, since
# pi only appends. Anything else that differs is the rewrite's doing.
written_since() {
  local was_length was_time now_length now_time
  read -r was_length was_time _ <<< "$1"
  read -r now_length now_time _ <<< "$2"
  [ -n "${now_length:-}" ] && [ "$now_length" -gt "$was_length" ] && [ "$now_time" -gt "$was_time" ]
}

# The logs to look at, found once before any is rewritten, so the walk
# never meets a file it has just written. One gone mid-walk is passed over.
logs=$(mktemp)
trap 'rm -f "$logs"' EXIT
find "$sessions" -ignore_readdir_race -name '*.jsonl' -mtime +"$whole_days_past" -size +"$smallest" -print0 > "$logs"

before=$(used)
rewritten=0
failed=0
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
  # Said once a log, so a run the clock ends while it waits reads as
  # waiting in the journal, not wedged.
  if disk_waits; then
    echo "waiting while other work waits on the disk, before $log"
    while disk_waits; do
      sleep 5
    done
  fi
  # A session reopened while the run waited writes to its log again, so
  # it is left for a night it's closed.
  still_closed "$log" || continue
  was=$(seen "$log") || continue
  if ! btrfs filesystem defragment -czstd "$log"; then
    echo "couldn't rewrite $log compressed; it stays as it was" >&2
    failed=$((failed + 1))
    continue
  fi
  # Written out now, a log at a time, rather than all at once later. A log
  # gone since its rewrite is passed over.
  sync -- "$log" || continue
  # The written pages stay in memory, where a read would find them; asked
  # to drop them, the kernel reads the log back from disk. One that won't
  # leaves the read to memory, which still sees a write meanwhile.
  dd if="$log" iflag=nocache count=0 status=none || :
  now=$(seen "$log") || now=
  if written_since "$was" "$now"; then
    echo "$log was written to during its rewrite; it's left for a night it's closed"
    continue
  fi
  if [ "$now" != "$was" ]; then
    echo "$log read $was before its rewrite and ${now:-nothing} after" >&2
    failed=$((failed + 1))
    continue
  fi
  rewritten=$((rewritten + 1))
done < "$logs"
after=$(used)
echo "rewrote $rewritten logs compressed; $sessions's file system went from $before to $after bytes used"
if [ "$failed" -gt 0 ]; then
  echo "$failed logs couldn't be rewritten, or read otherwise after" >&2
  exit 1
fi
