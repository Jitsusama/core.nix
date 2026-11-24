# shellcheck shell=bash
# Sets a disk's writeback throttle goal, or gives back what it had before.
# Usage: writeback-goal <disk> <microseconds>, where the disk is its name
# under /sys/block and 0 turns the throttle off; writeback-goal <disk>
# kernel gives it back the goal it read before the first goal was set. Its
# help and failures take the shape every command of ours does
# (docs/commands.md), by hand. WRITEBACK_GOAL_SYSFS puts sysfs under
# another folder, and WRITEBACK_GOAL_RUN the record of what each disk had,
# for its test.
case ${1:-} in
-h | --help)
  cat << 'EOF'
writeback-goal  sets a disk's writeback throttle goal

EXAMPLES
  writeback-goal nvme0n1 0       turn its throttle off
  writeback-goal nvme0n1 75000   hold writeback to 75 ms
  writeback-goal nvme0n1 kernel  the goal it had before

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
held=${WRITEBACK_GOAL_SYSFS:-}/sys/block/$disk/queue/wbt_lat_usec
record=${WRITEBACK_GOAL_RUN:-/run/writeback-goal}/$disk
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
  # What the disk had before the first goal was set comes back, so a
  # throttle BFQ or io.cost stood down, which reads 0, stays down. With no
  # record, as after a boot, -1 is the kernel's default goal, set by hand:
  # a later io.cost won't stand it down until the next boot.
  if [ -e "$record" ]; then
    cat "$record" > "$held"
    rm -f "$record"
  else
    echo -1 > "$held"
  fi
  exit 0
fi
# Only the first goal's record is kept, so a goal set twice gives back
# the disk's own, not the first goal.
if [ ! -e "$record" ]; then
  mkdir -p "${record%/*}"
  cat "$held" > "$record"
fi
echo "$goal" > "$held"
