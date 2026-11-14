# shellcheck shell=bash # writeShellApplication gives it bash.
# Grows the swap file to the size the machine asks for, as it starts and
# before anything swaps to it: disko makes the file only when it installs,
# so a machine whose swap size grew since kept the smaller one. It never
# shrinks one, never touches one in use or holding a hibernation not yet
# read back, and keeps the old one when the disk hasn't room for the new.
#
#   swap-file-grows FILE SIZE   SIZE as disko takes it, such as 128G
#
# SWAPS and EFIVARS name where the kernel lists what it swaps to and where
# the firmware's variables are, for its check to stand in for. Its help and
# failures take the shape every command of ours does (docs/commands.md), by
# hand.
set -euo pipefail

case "${1:-}" in
-h | --help)
  cat << 'EOF'
swap-file-grows  grow the swap file to the size asked

  Run as the machine starts, before anything swaps. It
  never shrinks one, nor touches one in use or holding a
  hibernation, and keeps the old one when the disk is
  short. SIZE is as disko takes it.

EXAMPLES
  swap-file-grows /swap/swapfile 128G

EXIT
  0   grown, or left as it is
  64  asked wrongly
  else what failed: numfmt, df or btrfs
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -ne 2 ]; then
  printf '%s\n' "swap-file-grows takes the file and its size" \
    "  try  swap-file-grows /swap/swapfile 128G" >&2
  exit 64
fi

swaps=${SWAPS:-/proc/swaps}
efivars=${EFIVARS:-/sys/firmware/efi/efivars}

file=$1
size=$2
want=$(numfmt --from=iec "$size")
have=$(stat -c %s "$file" 2>/dev/null || echo 0)
# Room left on the disk beside the new file, so the disk guard's line
# still holds once it's made.
spare=$((20 * 1024 * 1024 * 1024))

if [ "$have" -ge "$want" ]; then
  exit 0
fi
if grep -q "^$file " "$swaps"; then
  echo "$file is in use, so it stays at $have bytes until the next start"
  exit 0
fi
# systemd says where an image it wrote went in this variable, and clears
# it once the image is read back or thrown away.
for variable in "$efivars"/HibernateLocation-*; do
  if [ -e "$variable" ]; then
    echo "$file may hold a hibernation not yet read back, so it stays at $have bytes"
    exit 0
  fi
done
free=$(df --output=avail -B1 "$(dirname "$file")" | tail -n 1)
if [ $((free + have)) -lt $((want + spare)) ]; then
  echo "the disk has $free bytes free, too few to grow $file to $size, so it stays at $have bytes"
  exit 0
fi

rm -f "$file"
if btrfs filesystem mkswapfile --size "$size" "$file"; then
  echo "$file grew from $have bytes to $size"
elif [ "$have" -gt 0 ]; then
  echo "$file couldn't grow to $size, so it's made again at $have bytes"
  btrfs filesystem mkswapfile --size "$have" "$file"
fi
