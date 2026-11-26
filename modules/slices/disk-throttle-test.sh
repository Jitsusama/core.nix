# shellcheck shell=bash
# Proves writeback-goal and disk-order on a stand-in sysfs: writeback-goal
# refuses a wrong call before it looks for the disk, fails on a name that
# reaches no throttle, gives back nothing quietly for a disk that's gone,
# and gives back what the disk had before its first goal, or the kernel's
# default where nothing whole was recorded. A throttle that can't be read
# is recorded as off; a goal the throttle won't take fails, dropping only
# the record that call made; a giving back it won't take fails and keeps
# its record; and a name is one disk's, of letters and digits. disk-order
# stands the throttle down as io.cost takes the disk and gives the
# kernel's default back as it hands it back.
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

# says <text> <program> <words...>: the program, given the words, ends
# well and says the text on its standard error; with no text, it says
# nothing there. What it said is kept in said, as ends keeps it, so a
# holds after either reads that call's words.
says() {
  local want=$1
  shift
  said=$("$@" 2>&1 > /dev/null) || {
    echo "${*##*/} failed, saying '$said'"
    exit 1
  }
  if [[ -z $want && -n $said ]] || [[ $said != *"$want"* ]]; then
    echo "${*##*/} said '$said', not '$want'"
    exit 1
  fi
}

# ends <code> <program> <words...>: the program, given the words, ends
# with the code, and what it said on its standard error is kept in said.
ends() {
  local want=$1 got=0
  shift
  said=$("$@" 2>&1 > /dev/null) || got=$?
  if [[ $got != "$want" ]]; then
    echo "${*##*/} exited $got, not $want, saying '$said'"
    exit 1
  fi
}

# holds <text>: what the program ends kept last said includes the text.
holds() {
  if [[ $said != *"$1"* ]]; then
    echo "it said '$said', not '$1'"
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
# A record that isn't a number, empty or written by something else, or
# one too long for any goal, gives the kernel's default back rather than
# nothing, and goes with it.
for kept in "" 20x 99999999999; do
  echo 0 > "$throttle"
  printf %s "$kept" > "$WRITEBACK_GOAL_RUN/nvme0n1"
  says "record reads '$kept', so it gets the kernel's default" "$goal" nvme0n1 kernel
  reads "$throttle" -1
  if [[ -e $WRITEBACK_GOAL_RUN/nvme0n1 ]]; then
    echo "a record reading '$kept' outlived its giving back"
    exit 1
  fi
done
# A record of ten digits, the longest a start records, is the disk's own.
echo 0 > "$throttle"
printf %s 9999999999 > "$WRITEBACK_GOAL_RUN/nvme0n1"
exits 0 "$goal" nvme0n1 kernel
reads "$throttle" 9999999999
# A throttle read past ten digits is recorded as the kernel's default,
# since the giving back wouldn't take that record as the disk's own.
echo 99999999999 > "$throttle"
says "throttle reads '99999999999', so the kernel's default is recorded" "$goal" nvme0n1 0
reads "$WRITEBACK_GOAL_RUN/nvme0n1" -1
exits 0 "$goal" nvme0n1 kernel
reads "$throttle" -1
# A throttle that reads as no number is recorded as the kernel's default,
# so a second goal doesn't record the first and give that back.
echo busy > "$throttle"
says "throttle reads 'busy', so the kernel's default is recorded" "$goal" nvme0n1 0
reads "$WRITEBACK_GOAL_RUN/nvme0n1" -1
exits 0 "$goal" nvme0n1 75000
reads "$throttle" 75000
exits 0 "$goal" nvme0n1 kernel
reads "$throttle" -1
# A throttle that can't be read is recorded as off, as a queue with no
# throttle reads, so giving back doesn't turn one on; its goal is still
# set. Root reads past a mode, so the stand-in can't refuse it there.
echo 2000 > "$throttle"
chmod a-r "$throttle"
if [[ -r $throttle ]]; then
  echo "this runs as root, where a stand-in throttle can't be made unreadable"
  exit 1
fi
ends 0 "$goal" nvme0n1 75000
holds "throttle can't be read, so it's recorded as off"
holds "Permission denied"
chmod u+r "$throttle"
reads "$WRITEBACK_GOAL_RUN/nvme0n1" 0
reads "$throttle" 75000
exits 0 "$goal" nvme0n1 kernel
reads "$throttle" 0
# A throttle that reads but takes no goal fails the unit, saying so, and
# the record that start made goes, so a later start reads the disk afresh.
echo 2000 > "$throttle"
chmod a-w "$throttle"
ends 1 "$goal" nvme0n1 0
holds "throttle won't take 0"
chmod u+w "$throttle"
if [[ -e $WRITEBACK_GOAL_RUN/nvme0n1 ]]; then
  echo "a record outlived the goal it was made for failing"
  exit 1
fi
# A record an earlier goal made stays through a later goal's failing, and
# through a giving back the throttle won't take, so the disk's own still
# comes back once it will.
exits 0 "$goal" nvme0n1 0
chmod a-w "$throttle"
ends 1 "$goal" nvme0n1 75000
holds "throttle won't take 75000"
ends 1 "$goal" nvme0n1 kernel
holds "throttle won't take 2000 back; $goal nvme0n1 kernel tries again"
chmod u+w "$throttle"
reads "$WRITEBACK_GOAL_RUN/nvme0n1" 2000
exits 0 "$goal" nvme0n1 kernel
reads "$throttle" 2000
# A record of the kernel's default is taken as written, with nothing to
# say.
echo 0 > "$throttle"
echo -1 > "$WRITEBACK_GOAL_RUN/nvme0n1"
says "" "$goal" nvme0n1 kernel
reads "$throttle" -1
# A name that isn't one disk's is a wrong call, so nothing outside the
# records, or hidden among them, is removed.
echo kept > "$PWD/nvme0n1"
exits 64 "$goal" ../nvme0n1 kernel
reads "$PWD/nvme0n1" kept
echo kept > "$WRITEBACK_GOAL_RUN/.nvme0n1"
exits 64 "$goal" .nvme0n1 kernel
reads "$WRITEBACK_GOAL_RUN/.nvme0n1" kept
mkdir -p "$WRITEBACK_GOAL_RUN/a"
echo kept > "$WRITEBACK_GOAL_RUN/a/nvme0n1"
exits 64 "$goal" a/nvme0n1 kernel
reads "$WRITEBACK_GOAL_RUN/a/nvme0n1" kept
echo kept > "$WRITEBACK_GOAL_RUN/nvme0n1.new"
exits 64 "$goal" nvme0n1.new kernel
reads "$WRITEBACK_GOAL_RUN/nvme0n1.new" kept
rm -r "$WRITEBACK_GOAL_RUN/a" "$WRITEBACK_GOAL_RUN/nvme0n1.new"
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
