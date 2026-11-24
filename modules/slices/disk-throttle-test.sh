# shellcheck shell=bash
# Proves writeback-goal and disk-order on a stand-in sysfs: writeback-goal
# refuses a wrong call before it looks for the disk, fails on a name that
# reaches no throttle, gives back nothing quietly for a disk that's gone,
# and gives back what the disk had before its first goal, or the kernel's
# default where nothing whole was recorded; disk-order stands the throttle down
# as io.cost takes the disk and gives the kernel's default back as it hands
# it back.
# Usage: disk-throttle-test <writeback-goal> <disk-order>, run in an empty
# folder, where it builds the stand-in sysfs.
set -euo pipefail
goal=$1
order=$2
sys=$PWD/sys
export WRITEBACK_GOAL_SYSFS=$PWD DISK_ORDER_SYSFS=$PWD
export WRITEBACK_GOAL_RUN=$PWD/run

# disk <name> <throttle>: a whole disk, with its number and its throttle.
disk() {
  mkdir -p "$sys/block/$1/queue" "$sys/class/block/$1"
  echo "$2" > "$sys/block/$1/queue/wbt_lat_usec"
  echo 259:0 > "$sys/class/block/$1/dev"
}

# exits <code> <program> <words...>: the program, given the words, ends
# with the code.
exits() {
  local want=$1 got=0
  shift
  "$@" > /dev/null 2>&1 || got=$?
  if [[ $got != "$want" ]]; then
    echo "${*##*/} exited $got, not $want"
    exit 1
  fi
}

# reads <file> <want>: the file holds the line.
reads() {
  local got
  got=$(< "$1")
  if [[ $got != "$2" ]]; then
    echo "${1#"$PWD"/} reads '$got', not '$2'"
    exit 1
  fi
}

throttle=$sys/block/nvme0n1/queue/wbt_lat_usec

# A wrong call says so whether or not the disk is there.
exits 64 "$goal" nvme0n1 soon
exits 64 "$goal" nvme0n1
exits 64 "$goal" "" 0
# A name that reaches no throttle, such as a partition's, fails, and one
# gone by the time it's given back has nothing to give back.
exits 1 "$goal" nvme0n1 0
disk nvme0n1 2000
mkdir -p "$sys/class/block/nvme0n1p1"
exits 1 "$goal" nvme0n1p1 0
exits 0 "$goal" nvme0n1p1 kernel

# The goal is set, and what the disk had comes back on kernel, however
# many goals came between.
exits 0 "$goal" nvme0n1 0
reads "$throttle" 0
exits 0 "$goal" nvme0n1 75000
reads "$throttle" 75000
exits 0 "$goal" nvme0n1 kernel
reads "$throttle" 2000
if [[ -e $WRITEBACK_GOAL_RUN/nvme0n1 ]]; then
  echo "the record of nvme0n1's own goal outlived its giving back"
  exit 1
fi
# A throttle stood down before the first goal, as BFQ's is, stays down.
echo 0 > "$throttle"
exits 0 "$goal" nvme0n1 75000
exits 0 "$goal" nvme0n1 kernel
reads "$throttle" 0
# With nothing recorded, as after a boot, the kernel's default comes back.
exits 0 "$goal" nvme0n1 kernel
reads "$throttle" -1
# A record left empty or cut short, as by a kill mid-write, gives the
# kernel's default back rather than nothing, and goes with it.
for kept in "" 20x; do
  echo 0 > "$throttle"
  printf %s "$kept" > "$WRITEBACK_GOAL_RUN/nvme0n1"
  exits 0 "$goal" nvme0n1 kernel
  reads "$throttle" -1
  if [[ -e $WRITEBACK_GOAL_RUN/nvme0n1 ]]; then
    echo "a record reading '$kept' outlived its giving back"
    exit 1
  fi
done
# A throttle that reads as no number isn't recorded, so its giving back
# is the kernel's default too.
echo busy > "$throttle"
exits 0 "$goal" nvme0n1 0
reads "$throttle" 0
exits 0 "$goal" nvme0n1 kernel
reads "$throttle" -1
# A disk gone by its giving back drops its record, so a disk taking its
# name later records its own goal rather than giving back the old one's.
disk sdy 4000
exits 0 "$goal" sdy 0
mv "$sys/block/sdy" "$PWD/sdy-gone"
exits 0 "$goal" sdy kernel
disk sdy 8000
exits 0 "$goal" sdy 0
exits 0 "$goal" sdy kernel
reads "$sys/block/sdy/queue/wbt_lat_usec" 8000

# disk-order stands the throttle down beside io.cost, whatever stopped
# before it, clears a latency target left on the disk, and gives the
# kernel's default back as it hands the disk back.
mkdir -p "$sys/fs/cgroup/app.slice"
echo "259:0 target=1000" > "$sys/fs/cgroup/app.slice/io.latency"
echo 2000 > "$throttle"
exits 0 "$order" on nvme0n1 rbps=1 wlat=5000
reads "$sys/fs/cgroup/io.cost.model" "259:0 ctrl=user model=linear rbps=1"
reads "$sys/fs/cgroup/io.cost.qos" "259:0 enable=1 wlat=5000"
reads "$throttle" 0
reads "$sys/fs/cgroup/app.slice/io.latency" "259:0 target=max"
exits 0 "$order" off nvme0n1
reads "$sys/fs/cgroup/io.cost.qos" "259:0 enable=0"
reads "$throttle" -1
exits 0 "$order" off sdz
exits 64 "$order" sideways nvme0n1
echo "every case held"
