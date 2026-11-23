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
# is ended: btrfs balance start sets no handler for timeout's signals, so
# they kill it, and the kernel cancels a balance whose process is dying. A step that fails is said and the next is
# tried, since a fuller step can find room an emptier one didn't. Its help and failures take the
# shape every command of ours does (docs/commands.md), by hand.
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
  0   enough room, or outside the night's hours
  1   still short after the last step
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
    echo "btrfs filesystem usage -b $mount gave no $1 in bytes, so nothing is balanced" >&2
    exit 65
  fi
  echo "$bytes"
}

# Reads into left the seconds until 06:00, or 0 outside 01:00 to 06:00,
# so the night's hours can be checked without a function in a condition,
# where set -e is off.
read_clock() {
  local now hours minutes seconds since
  now=$(date +%H:%M:%S)
  IFS=: read -r hours minutes seconds <<<"$now"
  since=$((10#$hours * 3600 + 10#$minutes * 60 + 10#$seconds))
  left=0
  if [ "$since" -ge 3600 ] && [ "$since" -lt 21600 ]; then
    left=$((21600 - since))
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
echo "$((free / gib)) GiB of $mount is in no chunk, short of $((want / gib)), so data chunks are balanced"
ran=0
for full in 0 5 10 20 30 40 50; do
  read_clock
  if [ "$left" -eq 0 ]; then
    echo "it's past 06:00 with $((free / gib)) GiB in no chunk, so the rest waits for the next night"
    exit 1
  fi
  status=0
  timeout --kill-after=60 "$left" btrfs balance start -dusage="$full" "$mount" || status=$?
  if [ "$status" -eq 0 ]; then
    ran=$((ran + 1))
  elif [ "$status" -eq 124 ] || [ "$status" -eq 137 ]; then
    # 137 is a step that outlived TERM by a minute and was killed.
    echo "balancing data chunks up to $full% full was ended at 06:00"
  else
    echo "balancing data chunks up to $full% full failed, so the next step is tried"
  fi
  free=$(usage "Device unallocated")
  if [ "$free" -ge "$want" ]; then
    echo "$((free / gib)) GiB is in no chunk now, after chunks up to $full% full"
    exit 0
  fi
done
if [ "$ran" -eq 0 ]; then
  echo "every balance step failed, so $((free / gib)) GiB is still in no chunk" >&2
else
  echo "$((free / gib)) GiB is in no chunk after chunks up to half full, so what's left needs files gone" >&2
fi
exit 1
