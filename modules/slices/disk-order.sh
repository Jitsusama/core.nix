# shellcheck shell=bash
# Shares a disk by the slices' io.weight, through the kernel's io.cost, or
# stops sharing it. Usage: disk-order on <disk> <model> <qos>, where the
# disk is its name under /sys/class/block and the model and qos are the
# key=value settings io.cost.model and io.cost.qos take; disk-order off
# <disk> hands the disk back to first come, first served. Its help and
# failures take the shape every command of ours does (docs/commands.md), by
# hand. DISK_ORDER_SYSFS puts sysfs under another folder, for its test.
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
  1   the kernel refused a setting
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac
mode=${1:-}
disk=${2:-}
sys=${DISK_ORDER_SYSFS:-}/sys
# A disk that's gone took its io.cost settings with it, so there's
# nothing to hand back.
if [ "$mode" = off ] && [ ! -e "$sys/class/block/$disk" ]; then
  exit 0
fi
number=$(cat "$sys/class/block/$disk/dev")
# The writeback throttle, which io.cost stands down as it turns on, but
# which a switch that moves the disk from jitsusama.slices.writebackThrottle
# into jitsusama.slices.disks gives back as the throttle's unit stops.
throttle=$sys/block/$disk/queue/wbt_lat_usec

case $mode in
on)
  # io.cost shares the disk by whatever model it holds as it turns on, and
  # the one it guesses is far off, so the measured one goes in first.
  echo "$number ctrl=user model=linear $3" > "$sys/fs/cgroup/io.cost.model"
  echo "$number enable=1 $4" > "$sys/fs/cgroup/io.cost.qos"
  # So it's down beside io.cost whatever stopped before this started.
  if [ -e "$throttle" ]; then
    echo 0 > "$throttle"
  fi
  # A measured disk has no latency targets, but a switch from before it was
  # measured leaves them in the kernel: systemd doesn't write over a target
  # it no longer manages. Left, they throttle the disk beside io.cost until
  # a reboot, so they go here.
  find "$sys/fs/cgroup" -name io.latency -print0 | while IFS= read -r -d '' held; do
    if grep -q "^$number " "$held"; then
      echo "$number target=max" > "$held"
    fi
  done
  ;;
off)
  echo "$number enable=0" > "$sys/fs/cgroup/io.cost.qos"
  # The kernel's default goal, -1: io.cost leaving brings back only a
  # throttle the kernel stood down itself (blk-iocost.c's
  # wbt_enable_default), never the 0 set by hand above.
  if [ -e "$throttle" ]; then
    echo -1 > "$throttle"
  fi
  ;;
*)
  printf '%s\n' "disk-order turns sharing on or off, not ${mode:-nothing}" \
    "  try  disk-order --help" >&2
  exit 64
  ;;
esac
