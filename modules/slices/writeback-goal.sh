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
# the disk is there. A name is held to letters and digits, as the module
# holds the disks it makes units for, which keeps a call off any path
# outside the records, off a record's half-written copy, <disk>.new, and
# out of the journal's lines as anything but one word.
if [[ ! $disk =~ ^[a-z0-9]+$ ]] \
  || { [ "$goal" != kernel ] && [[ -z $goal || $goal == *[!0-9]* ]]; }; then
  printf '%s\n' "writeback-goal sets a disk's goal in microseconds, or kernel" \
    "  try  writeback-goal --help" >&2
  exit 64
fi
if [ ! -e "$held" ]; then
  # A disk gone by the time it's given back has nothing to give back, and
  # its record goes, so a disk taking its name later records its own. One
  # that's missing as its goal is set is a name that reaches nothing,
  # which the unit's failing says rather than the A/B measuring nothing.
  if [ "$goal" = kernel ]; then
    rm -f "$record"
    exit 0
  fi
  echo "writeback-goal: $disk has no writeback throttle under /sys/block" >&2
  exit 1
fi
if [ "$goal" = kernel ]; then
  # What the disk had before the first goal was set comes back, so a
  # throttle BFQ or io.cost stood down, which reads 0, stays down. With no
  # record, as after a boot, -1 is the kernel's default goal, set by hand,
  # which disk-order stands down if io.cost takes the disk later. A record
  # that isn't a number gives the default back too, rather than nothing,
  # and so does one past ten digits, longer than any goal is (ten digits
  # of microseconds is hours) and longer than a start records.
  before=-1
  if [ -e "$record" ]; then
    kept=$(< "$record")
    if [[ $kept == -1 || $kept =~ ^[0-9]{1,10}$ ]]; then
      before=$kept
    else
      echo "writeback-goal: $disk's record reads '$kept', so it gets the kernel's default" >&2
    fi
  fi
  # A throttle that won't take it back keeps the record, so a later giving
  # back tries the disk's own again; the line says how, by this script's
  # own path, which no PATH holds, since a unit stopped for good brings no
  # later stop.
  if ! echo "$before" > "$held"; then
    echo "writeback-goal: $disk's throttle won't take $before back;" \
      "$0 $disk kernel tries again" >&2
    exit 1
  fi
  rm -f "$record"
  exit 0
fi
# Only the first goal's record is kept, so a goal set twice gives back
# the disk's own, not the first goal. It's written whole, through a file
# renamed into place, so a second goal can't be taken for the disk's own.
# A throttle that reads as no number, or one past ten digits, which the
# giving back wouldn't take as the disk's own, is recorded as the kernel's
# default.
# One that can't be read is recorded as off: as the block layer reads, a
# queue with no throttle set up refuses the read but takes a write, which
# sets one up, so giving back the default would turn on a throttle the
# disk never had; giving back 0 leaves it one set up and off, as near to
# none as sysfs comes. cat's own error goes to the journal, saying why.
made=
if [ ! -e "$record" ]; then
  made=1
  if ! now=$(cat "$held"); then
    echo "writeback-goal: $disk's throttle can't be read, so it's recorded as off" >&2
    now=0
  elif [[ ! $now =~ ^[0-9]{1,10}$ ]]; then
    echo "writeback-goal: $disk's throttle reads '$now', so the kernel's default is recorded" >&2
    now=-1
  fi
  mkdir -p "${record%/*}"
  echo "$now" > "$record.new"
  mv "$record.new" "$record"
fi
# A goal the throttle won't take fails the unit, and the record this call
# made goes with it, so a later start reads the disk afresh rather than
# giving back a record of a goal never set. A record an earlier goal made
# stays, since it still holds the disk's own.
if ! echo "$goal" > "$held"; then
  if [ -n "$made" ]; then
    rm -f "$record"
  fi
  echo "writeback-goal: $disk's throttle won't take $goal" >&2
  exit 1
fi
