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
# It balances only from 01:00 to 06:00, checked before each step, since a
# laptop asleep at night fires its timer as it wakes and a switch can
# fire it at any hour. A step that fails is said and the next is tried,
# since a fuller step can find room an emptier one didn't. HOUR stands in
# for the clock's hour, for its check. Its help and failures take the
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
  2   btrfs's usage couldn't be read
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
    exit 2
  fi
  echo "$bytes"
}

# Whether the clock is in the night's hours, when a balance may run.
at_night() {
  local hour
  hour=$((10#${HOUR:-$(date +%H)}))
  [ "$hour" -ge 1 ] && [ "$hour" -lt 6 ]
}

gib() {
  echo $(($1 / 1024 / 1024 / 1024))
}

if ! at_night; then
  echo "it's outside 01:00 to 06:00, so nothing is balanced until the night"
  exit 0
fi

size=$(usage "Device size")
cap=$((50 * 1024 * 1024 * 1024))
want=$((size / 10))
if [ "$want" -gt "$cap" ]; then
  want=$cap
fi

free=$(usage "Device unallocated")
if [ "$free" -ge "$want" ]; then
  exit 0
fi
echo "$(gib "$free") GiB of $mount is in no chunk, short of $(gib "$want"), so data chunks are balanced"
ran=0
for full in 0 5 10 20 30 40 50; do
  if ! at_night; then
    echo "it's past 06:00 with $(gib "$free") GiB in no chunk, so the rest waits for the next night"
    exit 1
  fi
  if btrfs balance start -dusage="$full" "$mount"; then
    ran=$((ran + 1))
  else
    echo "balancing data chunks up to $full% full failed, so the next step is tried"
  fi
  free=$(usage "Device unallocated")
  if [ "$free" -ge "$want" ]; then
    echo "$(gib "$free") GiB is in no chunk now, after chunks up to $full% full"
    exit 0
  fi
done
if [ "$ran" -eq 0 ]; then
  echo "every balance step failed, so $(gib "$free") GiB is still in no chunk" >&2
else
  echo "$(gib "$free") GiB is in no chunk after chunks up to half full, so what's left needs files gone" >&2
fi
exit 1
