# shellcheck shell=bash
# Makes the camera's loopback, or takes the one already there, and writes
# its node to a file for the relay and the holders watch to read.
#
# The relay's stop deletes its loopback, but a delete fails while a program
# still holds it open, as a call or a test left waiting on a camera giving no
# frames does, and each start used to add another. The one left behind
# stayed in every program's list of cameras with no picture, and Chrome
# keeps a camera by its node, so a call could take the dead one. Taking the
# loopback already there keeps its node across restarts, and a program
# still holding it sees frames again once the relay writes to it. Any other
# of the same name is one an earlier start left behind, deleted where it can
# be; a held one is tried again at the next start.
#
# Usage: camera-loopback <name> <file> <v4l2loopback-ctl add's options>...
# LOOPBACK_SYSFS names the folder the loopbacks are listed in, for tests.
set -euo pipefail

name=$1
file=$2
shift 2
sysfs=${LOOPBACK_SYSFS:-/sys/devices/virtual/video4linux}

# The loopbacks of this name, oldest first, since the oldest is the one a
# program kept from before is likeliest to name.
mapfile -t nodes < <(
  for card in "$sysfs"/video*/name; do
    if [ -e "$card" ] && [ "$(cat "$card")" = "$name" ]; then
      basename "$(dirname "$card")"
    fi
  done | sort -V
)

if [ "${#nodes[@]}" -eq 0 ]; then
  kept=$(v4l2loopback-ctl add -n "$name" "$@")
else
  kept=/dev/${nodes[0]}
  echo "Taking $name at $kept, already there."
  for node in "${nodes[@]:1}"; do
    if v4l2loopback-ctl delete "/dev/$node"; then
      echo "Deleted /dev/$node, a $name left from before."
    else
      echo "/dev/$node, a $name left from before, is still held; trying again next start."
    fi
  done
fi

echo "$kept" >"$file"
