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
# What it's asked is checked first, so a wrong call says so whether or not
# the disk is there.
if [ -z "$disk" ] || { [ "$goal" != kernel ] && [[ -z $goal || $goal == *[!0-9]* ]]; }; then
  printf '%s\n' "writeback-goal sets a disk's goal in microseconds, or kernel" \
    "  try  writeback-goal --help" >&2
  exit 64
fi
if [ ! -e "$held" ]; then
  # A disk gone by the time it's given back has nothing to give back, but
  # one that's missing as its goal is set is a name that reaches nothing,
  # which the unit's failing says rather than the A/B measuring nothing.
  [ "$goal" = kernel ] && exit 0
  echo "writeback-goal: $disk has no writeback throttle under /sys/block" >&2
  exit 1
fi
if [ "$goal" = kernel ]; then
  # The kernel reads -1 as its default goal, set by hand: the goal comes
  # back, but a later io.cost won't stand it down until the next boot.
  goal=-1
fi
echo "$goal" > "$held"
