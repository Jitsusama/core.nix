# shellcheck shell=bash # writeShellApplication gives it bash.
# Gives Btrfs back room it can put metadata in. Btrfs hands the disk out
# in chunks, each for data or for metadata, and a data chunk emptied by
# deletes stays a data chunk. Once every byte is in some chunk, metadata
# can't grow, and writes fail as the disk being full with many gigabytes
# free inside data chunks, after stalling as Btrfs hunts for room. So,
# while less than a tenth of the disk (at most 50 GiB) is in no chunk,
# this balances the emptiest data chunks, a step fuller each time, until
# enough is free or the chunks left are half full.
#
#   chunks-given-back MOUNT   MOUNT a Btrfs file system's mount point
#
# It balances only from 01:00 to 06:00, since a laptop asleep at night
# fires its timer as it wakes and a switch can fire it at any hour: it
# starts no step outside those hours, and a step still running at 06:00
# is ended: btrfs balance start sets no handler for timeout's signals,
# so they kill it, and the kernel cancels a balance whose process is
# dying. A step that fails, or is killed by something else before 06:00,
# is said and the next is tried, since a fuller step can find room an
# emptier one didn't. A night left short only because the data chunks
# are full, as the half-full step finishing or failing for want of room
# shows, or because 06:00 came, says so and ends well: each is a state
# it was built to meet, which deleting files or the next night answers,
# and failing for it marked the unit failed every such night for nothing
# it could mend. Its help and failures take the shape every command of
# ours does (docs/commands.md), by hand.
set -euo pipefail

case "${1:-}" in
-h | --help)
  cat << 'EOF'
chunks-given-back  give Btrfs room for its metadata

  Balances the emptiest data chunks until a tenth of
  the disk, at most 50 GiB, is in no chunk, or the
  chunks left are half full. Only from 01:00 to 06:00.

EXAMPLES
  chunks-given-back /

EXIT
  0   enough room, outside the night's hours, 06:00
      came, or short only for full data chunks
  1   no step finished, or the half-full one was killed
      or failed for anything but room, before 06:00
  65  btrfs's usage couldn't be read
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -ne 1 ]; then
  printf '%s\n' "chunks-given-back takes the mount point" \
    "  try  chunks-given-back /" >&2
  exit 64
fi

mount=$1

# The bytes Btrfs says of the line named, from its usage in bytes. awk
# reads to the end, so btrfs never writes into a closed pipe.
usage() {
  local bytes
  bytes=$(btrfs filesystem usage -b "$mount" | awk -v line="$1:" '
    { sub(/^[ \t]+/, "") }
    !found && index($0, line) == 1 { sub(line, ""); print $1; found = 1 }
  ')
  if ! [[ $bytes =~ ^[0-9]+$ ]]; then
    echo "btrfs filesystem usage -b $mount gave no $1 in bytes," \
      "so nothing is balanced" >&2
    exit 65
  fi
  echo "$bytes"
}

# Reads into left the seconds until 06:00, or 0 outside 01:00 to 06:00,
# so the night's hours can be checked without a function in a condition,
# where set -e is off. The seconds are counted to 06:00 by the epoch, so
# a night the clocks go forward still ends its step at 06:00.
read_clock() {
  local now epoch six hours minutes seconds since
  now=$(date '+%H:%M:%S %s')
  read -r now epoch <<<"$now"
  IFS=: read -r hours minutes seconds <<<"$now"
  since=$((10#$hours * 3600 + 10#$minutes * 60 + 10#$seconds))
  left=0
  if [ "$since" -ge 3600 ] && [ "$since" -lt 21600 ]; then
    six=$(date -d 06:00 +%s)
    if [ "$six" -gt "$epoch" ]; then
      left=$((six - epoch))
    fi
  fi
}

gib=$((1024 * 1024 * 1024))

read_clock
if [ "$left" -eq 0 ]; then
  echo "it's outside 01:00 to 06:00, so nothing is balanced until the night"
  exit 0
fi

size=$(usage "Device size")
cap=$((50 * gib))
want=$((size / 10))
if [ "$want" -gt "$cap" ]; then
  want=$cap
fi

free=$(usage "Device unallocated")
if [ "$free" -ge "$want" ]; then
  exit 0
fi
echo "$((free / gib)) GiB of $mount is in no chunk, short of $((want / gib))," \
  "so data chunks are balanced"
ran=0
killed=0
failed=0
for full in 0 5 10 20 30 40 50; do
  read_clock
  if [ "$left" -eq 0 ]; then
    echo "it's past 06:00 with $((free / gib)) GiB in no chunk," \
      "so the rest waits for the next night"
    exit 0
  fi
  status=0
  ended=0
  killed=0
  failed=0
  onward=", so the next step is tried"
  if [ "$full" -eq 50 ]; then
    onward=""
  fi
  # What btrfs says is kept to tell a step that failed for want of room,
  # the one failure that shows the chunks are full, and said after it.
  said=$(timeout --kill-after=60 "$left" btrfs balance start -dusage="$full" "$mount" 2>&1) ||
    status=$?
  if [ -n "$said" ]; then
    printf '%s\n' "$said"
  fi
  if [ "$status" -eq 0 ]; then
    ran=$((ran + 1))
  elif [ "$status" -eq 124 ]; then
    ended=1
    echo "balancing data chunks up to $full% full was ended at 06:00"
  elif [ "$status" -eq 137 ]; then
    # Any SIGKILL gives 137: timeout's a minute after TERM, or the
    # kernel's OOM killer's at any hour, so only the clock says which.
    # A TERM from anything but timeout gives 143, said as a failure.
    read_clock
    if [ "$left" -eq 0 ]; then
      ended=1
      echo "balancing data chunks up to $full% full was killed after 06:00"
    else
      killed=1
      echo "balancing data chunks up to $full% full was killed$onward"
    fi
  elif [[ $said == *"No space left on device"* ]]; then
    echo "balancing data chunks up to $full% full failed for want of room$onward"
  else
    failed=1
    echo "balancing data chunks up to $full% full failed$onward"
  fi
  free=$(usage "Device unallocated")
  if [ "$free" -ge "$want" ]; then
    echo "$((free / gib)) GiB is in no chunk now, after chunks up to $full% full"
    exit 0
  fi
  if [ "$ended" -eq 1 ]; then
    echo "it's past 06:00 with $((free / gib)) GiB in no chunk," \
      "so the rest waits for the next night"
    exit 0
  fi
done
if [ "$ran" -eq 0 ]; then
  echo "no balance step finished, so $((free / gib)) GiB is still in no chunk" >&2
  exit 1
elif [ "$killed" -eq 1 ]; then
  # Files gone would be guessing: the half-full step that would have
  # said so was killed part way. One that failed, as for want of room,
  # did say so.
  echo "balancing chunks up to half full didn't finish, so $((free / gib)) GiB" \
    "is still in no chunk" >&2
  exit 1
elif [ "$failed" -eq 1 ]; then
  # Only a failure for want of room says the chunks are full; any other
  # leaves it unknown, and may be a fault worth seeing.
  echo "balancing chunks up to half full failed, so $((free / gib)) GiB" \
    "is still in no chunk" >&2
  exit 1
fi
# The chunks left are at least half full, so only deletes free more: the
# disk guard's work, which a failed unit here would only hide behind.
echo "$((free / gib)) GiB is in no chunk after chunks up to half full," \
  "so what's left needs files gone"
exit 0
