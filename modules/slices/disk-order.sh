# shellcheck shell=bash
# Shares a disk by the slices' io.weight, through the kernel's io.cost, or
# stops sharing it. Usage: disk-order on <disk> <model> <qos>, where the
# disk is its name under /sys/class/block and the model and qos are the
# key=value settings io.cost.model and io.cost.qos take; disk-order off
# <disk> hands the disk back to first come, first served.
mode=$1
disk=$2
# A disk that's gone took its io.cost settings with it, so there's
# nothing to hand back.
if [ "$mode" = off ] && [ ! -e "/sys/class/block/$disk" ]; then
  exit 0
fi
number=$(cat "/sys/class/block/$disk/dev")

case $mode in
on)
  # io.cost shares the disk by whatever model it holds as it turns on, and
  # the one it guesses is far off, so the measured one goes in first.
  echo "$number ctrl=user model=linear $3" > /sys/fs/cgroup/io.cost.model
  echo "$number enable=1 $4" > /sys/fs/cgroup/io.cost.qos
  # A measured disk has no latency targets, but a switch from before it was
  # measured leaves them in the kernel: systemd doesn't write over a target
  # it no longer manages. Left, they throttle the disk beside io.cost until
  # a reboot, so they go here.
  find /sys/fs/cgroup -name io.latency -print0 | while IFS= read -r -d '' held; do
    if grep -q "^$number " "$held"; then
      echo "$number target=max" > "$held"
    fi
  done
  ;;
off)
  echo "$number enable=0" > /sys/fs/cgroup/io.cost.qos
  ;;
*)
  echo "disk-order: on or off, not $mode" >&2
  exit 2
  ;;
esac
