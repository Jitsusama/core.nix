# shellcheck shell=bash
# Proves efficient-cores.sh holds only the efficient cores' clock ceiling,
# within each core's own range, lets one hold stand at a time, and gives
# each core back the ceiling it had, even after a hold that failed partway.
# Usage: efficient-cores-test <efficient-cores>, run in an empty folder,
# where it builds a stand-in sysfs.
set -euo pipefail
cores=$1
sys=$PWD/sys
export EFFICIENT_CORES_SYSFS=$PWD
export EFFICIENT_CORES_RUN=$PWD/run

# core <n> <lowest kHz> <highest kHz> <ceiling kHz>: a core's cpufreq.
core() {
  local freq=$sys/devices/system/cpu/cpu$1/cpufreq
  mkdir -p "${freq}"
  echo "$2" >"${freq}/cpuinfo_min_freq"
  echo "$2" >"${freq}/scaling_min_freq"
  echo "$3" >"${freq}/cpuinfo_max_freq"
  echo "$4" >"${freq}/scaling_max_freq"
}

# ceilings <want>: every core's ceiling, in order, as one line.
ceilings() {
  local got="" n
  for n in 0 1 2 3 4 5; do
    got+="$(<"$sys/devices/system/cpu/cpu$n/cpufreq/scaling_max_freq") "
  done
  if [[ "${got% }" != "$1" ]]; then
    echo "the ceilings read '${got% }', not '$1'"
    exit 1
  fi
}

# exits <code> <words...>: the script, given the words, ends with the code.
exits() {
  local want=$1 got=0
  shift
  "${cores}" "$@" >/dev/null 2>&1 || got=$?
  if [[ "${got}" != "${want}" ]]; then
    echo "'$*' exited ${got}, not ${want}"
    exit 1
  fi
}

# Two performance cores, then three efficient ones, the last two of which
# reach less, as the laptop's low-power ones do, and a performance core
# between them, so the list's ranges are read rather than assumed. The
# first efficient core boots below its highest clock, as another writer
# may have set it, so giving back its ceiling differs from freeing it.
# The second can't run below 800 MHz, so the lowest speed asked for shows
# a core held at its own lowest.
core 0 400000 4800000 4800000
core 1 400000 4700000 4700000
core 2 400000 3700000 3000000
core 3 800000 3300000 3300000
core 4 400000 4700000 4700000
core 5 400000 3300000 3300000
mkdir -p "$sys/devices/cpu_atom"
echo "2-3,5" >"$sys/devices/cpu_atom/cpus"
boot="4800000 4700000 3000000 3300000 4700000 3300000"

exits 0 hold 1600
ceilings "4800000 4700000 1600000 1600000 4700000 1600000"
# The same hold again stands as it was; another is refused, and another
# speed's stop leaves it, as a refused unit's stop does.
exits 0 hold 1600
exits 75 hold 2000
ceilings "4800000 4700000 1600000 1600000 4700000 1600000"
exits 0 free 2000
ceilings "4800000 4700000 1600000 1600000 4700000 1600000"
exits 0 free 1600
ceilings "${boot}"
exits 0 free 1600
ceilings "${boot}"

# Below a core's lowest clock it's held at its lowest, above its highest at
# its highest, so no ceiling is one the kernel would refuse.
exits 0 hold 400
ceilings "4800000 4700000 400000 800000 4700000 400000"
exits 0 free
exits 0 hold 5000
ceilings "4800000 4700000 3700000 3300000 4700000 3300000"
exits 0 free
ceilings "${boot}"

# A hold the kernel refuses partway fails, and its stop still gives back
# every ceiling it changed.
chmod a-w "$sys/devices/system/cpu/cpu5/cpufreq/scaling_max_freq"
exits 1 hold 1600
ceilings "4800000 4700000 1600000 1600000 4700000 3300000"
chmod u+w "$sys/devices/system/cpu/cpu5/cpufreq/scaling_max_freq"
exits 0 free 1600
ceilings "${boot}"

exits 0 --help
exits 64
exits 64 hold
exits 64 hold 16a
exits 64 hold 0
# Only a speed from 400 to 5000 MHz is held, which spans every core's
# clock today, and is the range the polkit grant allows.
exits 64 hold 399
exits 64 hold 5001
exits 64 hold 010
exits 64 hold 99999999999999999999
exits 64 hold 1600 more
exits 64 free 16a
exits 64 free 1600 more
exits 64 warm
ceilings "${boot}"

# A processor with no efficient cores holds nothing and fails, so a unit
# never reads as holding what it couldn't.
rm -r "$sys/devices/cpu_atom"
exits 69 hold 1600
ceilings "${boot}"
