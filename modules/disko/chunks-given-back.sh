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
# A step that fails is said and the next is tried, since a fuller step can
# find room an emptier one didn't. Its help and failures take the shape
# every command of ours does (docs/commands.md), by hand.
set -euo pipefail

case "${1:-}" in
-h | --help)
  cat << 'EOF'
chunks-given-back  give Btrfs room for its metadata

  Balances the emptiest data chunks until a tenth of
  the disk, at most 50 GiB, is in no chunk, or the
  chunks left are half full.

EXAMPLES
  chunks-given-back /

EXIT
  0   enough room, or as much as half-full chunks give
  64  asked wrongly
  else what failed: btrfs filesystem usage
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

# The bytes Btrfs says of the line named, from its usage in bytes.
usage() {
  btrfs filesystem usage -b "$mount" | awk -v line="$1:" '
    { sub(/^[ \t]+/, "") }
    index($0, line) == 1 { sub(line, ""); print $1 + 0; exit }
  '
}

size=$(usage "Device size")
cap=$((50 * 1024 * 1024 * 1024))
want=$((size / 10))
if [ "$want" -gt "$cap" ]; then
  want=$cap
fi

gib() {
  echo $(($1 / 1024 / 1024 / 1024))
}

free=$(usage "Device unallocated")
if [ "$free" -ge "$want" ]; then
  exit 0
fi
echo "$(gib "$free") GiB of $mount is in no chunk, short of $(gib "$want"), so data chunks are balanced"
for full in 0 5 10 20 30 40 50; do
  if ! btrfs balance start -dusage="$full" "$mount"; then
    echo "balancing data chunks up to $full% full failed, so the next step is tried"
  fi
  free=$(usage "Device unallocated")
  if [ "$free" -ge "$want" ]; then
    echo "$(gib "$free") GiB is in no chunk now, after chunks up to $full% full"
    exit 0
  fi
done
echo "$(gib "$free") GiB is in no chunk after chunks up to half full, so what's left needs files gone"
