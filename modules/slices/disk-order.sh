# shellcheck shell=bash
# Shares a disk by the slices' io.weight, through the kernel's io.cost, or
# stops sharing it. Usage: disk-order on <disk> <model> <qos>, where the
# disk is its name under /sys/class/block and the model and qos are the
# key=value settings io.cost.model and io.cost.qos take; disk-order off
# <disk> hands the disk back to first come, first served. Its help and
# failures take the shape every command of ours does (docs/commands.md), by
# hand. DISK_ORDER_SYSFS puts sysfs under another folder, and
# DISK_ORDER_RUN the record of what each disk's throttle read before
# io.cost took it, for its test.
case ${1:-} in
-h | --help)
  cat << 'EOF'
disk-order  shares a disk by the slices' io.weight

EXAMPLES
  disk-order on DISK MODEL QOS  share it, with io.cost's
                                measured settings
  disk-order off DISK           first come, first served

EXIT
  0   done
  1   the kernel refused a setting, or the disk isn't there
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac
mode=${1:-}
disk=${2:-}
# What it's asked is checked first, so a wrong call says so whether or not
# the disk is there, and a name that isn't one disk's reaches no path.
if [[ -z $disk || $disk == */* || $disk == .* ]] \
  || { [ "$mode" = on ] && [ $# -ne 4 ]; } \
  || { [ "$mode" = off ] && [ $# -ne 2 ]; } \
  || [[ $mode != on && $mode != off ]]; then
  printf '%s\n' "disk-order turns a disk's sharing on, with its model and qos, or off" \
    "  try  disk-order --help" >&2
  exit 64
fi
sys=${DISK_ORDER_SYSFS:-}/sys
record=${DISK_ORDER_RUN:-/run/disk-order}/$disk
# The writeback throttle, which io.cost stands down as it turns on, but
# which a switch that moves the disk from jitsusama.slices.writebackThrottle
# into jitsusama.slices.disks gives back as the throttle's unit stops.
throttle=$sys/block/$disk/queue/wbt_lat_usec

# put <file> <value>: the kernel takes the value, or the refusal says
# which setting it was and the run ends as the kernel refusing it.
put() {
  if ! printf '%s\n' "$2" 2> /dev/null > "$1"; then
    echo "disk-order: the kernel refused '$2' in ${1#"$sys"/} for $disk" >&2
    exit 1
  fi
}

if [ ! -e "$sys/class/block/$disk" ]; then
  # A disk that's gone took its io.cost settings with it, so there's
  # nothing to hand back, and its record goes, so a disk taking its name
  # later records its own throttle.
  if [ "$mode" = off ]; then
    rm -f "$record"
    exit 0
  fi
  echo "disk-order: $disk isn't a disk under /sys/class/block" >&2
  exit 1
fi
number=$(< "$sys/class/block/$disk/dev")

if [ "$mode" = on ]; then
  # What the throttle read before io.cost took the disk, so handing it back
  # gives that back: off where BFQ stood it down, the disk's goal where it
  # had one. Only the first is kept, so a restart doesn't record the 0 set
  # below, and it's written whole, through a file renamed into place.
  if [ -e "$throttle" ] && [ ! -e "$record" ]; then
    now=$(< "$throttle")
    if [[ -z $now || $now == *[!0-9]* ]]; then
      now=-1
    fi
    mkdir -p "${record%/*}"
    echo "$now" > "$record.new"
    mv "$record.new" "$record"
  fi
  # io.cost shares the disk by whatever model it holds as it turns on, and
  # the one it guesses is far off, so the measured one goes in first.
  put "$sys/fs/cgroup/io.cost.model" "$number ctrl=user model=linear $3"
  put "$sys/fs/cgroup/io.cost.qos" "$number enable=1 $4"
  # So it's down beside io.cost whatever stopped before this started.
  if [ -e "$throttle" ]; then
    put "$throttle" 0
  fi
  # A measured disk has no latency targets, but a switch from before it was
  # measured leaves them in the kernel: systemd doesn't write over a target
  # it no longer manages. Left, they throttle the disk beside io.cost until
  # a reboot, so they go here.
  find "$sys/fs/cgroup" -name io.latency -print0 | while IFS= read -r -d '' held; do
    if grep -q "^$number " "$held"; then
      put "$held" "$number target=max"
    fi
  done
  exit 0
fi

put "$sys/fs/cgroup/io.cost.qos" "$number enable=0"
# io.cost leaving brings back only a throttle the kernel stood down itself
# (blk-iocost.c's wbt_enable_default), never the 0 set by hand above, so
# what the throttle read before comes back: 0 where BFQ had it down. With
# no record whole, as after a boot, it's the kernel's default goal, -1.
if [ -e "$throttle" ]; then
  before=-1
  if [ -e "$record" ]; then
    kept=$(< "$record")
    if [[ $kept == -1 || (-n $kept && $kept != *[!0-9]*) ]]; then
      before=$kept
    fi
  fi
  put "$throttle" "$before"
fi
rm -f "$record"
