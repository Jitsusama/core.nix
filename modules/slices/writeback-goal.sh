# shellcheck shell=bash
# Sets a disk's writeback throttle goal, or gives the kernel's own back.
# Usage: writeback-goal <disk> <microseconds>, where the disk is its name
# under /sys/block and 0 turns the throttle off; writeback-goal <disk>
# kernel gives it the kernel's goal again. Its help and failures take the
# shape every command of ours does (docs/commands.md), by hand.
case ${1:-} in
-h | --help)
  cat << 'EOF'
writeback-goal  sets a disk's writeback throttle goal

EXAMPLES
  writeback-goal nvme0n1 0       turn its throttle off
  writeback-goal nvme0n1 75000   hold writeback to 75 ms
  writeback-goal nvme0n1 kernel  the kernel's own goal

EXIT
  0   done
  1   the kernel refused it
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac
disk=${1:-}
goal=${2:-}
held=/sys/block/$disk/queue/wbt_lat_usec
# A disk that's gone, or has no throttle, has nothing to set or give back.
if [ -z "$disk" ] || [ ! -e "$held" ]; then
  exit 0
fi
case $goal in
kernel)
  # The kernel reads -1 as its own goal for the disk.
  echo -1 > "$held"
  ;;
'' | *[!0-9]*)
  printf '%s\n' "writeback-goal sets microseconds or kernel, not ${goal:-nothing}" \
    "  try  writeback-goal --help" >&2
  exit 64
  ;;
*)
  echo "$goal" > "$held"
  ;;
esac
