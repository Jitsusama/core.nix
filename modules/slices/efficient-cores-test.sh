# shellcheck shell=bash
# Proves efficient-cores.sh holds only the efficient cores' clock ceiling,
# within each core's own range, lets one hold stand at a time, waits its
# turn behind another's lock, and frees each core to its highest clock,
# even after a hold or a free that failed partway.
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

# waits <words...>: the script, given the words, is still waiting after
# a second, which timeout says by ending it with 124.
waits() {
  local got=0
  timeout 1 "${cores}" "$@" >/dev/null 2>&1 || got=$?
  if [[ "${got}" != 124 ]]; then
    echo "'$*' exited ${got} rather than waiting its turn"
    exit 1
  fi
}

# Two performance cores, then three efficient ones, the last two of which
# reach less, as the laptop's low-power ones do, and a performance core
# between them, so the list's ranges are read rather than assumed. The
# first efficient core boots below its highest clock, as a thermal limit
# reads, so freeing it to its highest differs from leaving it as it was.
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
free="4800000 4700000 3700000 3300000 4700000 3300000"

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
ceilings "${free}"
exits 0 free 1600
ceilings "${free}"

# Below a core's lowest clock it's held at its lowest, above its highest at
# its highest, so no ceiling is one the kernel would refuse.
exits 0 hold 400
ceilings "4800000 4700000 400000 800000 4700000 400000"
exits 0 free
ceilings "${free}"
exits 0 hold 5000
ceilings "4800000 4700000 3700000 3300000 4700000 3300000"
exits 0 free
ceilings "${free}"

# A hold the kernel refuses partway fails, and its stop still frees every
# ceiling it changed.
chmod a-w "$sys/devices/system/cpu/cpu5/cpufreq/scaling_max_freq"
exits 1 hold 1600
ceilings "4800000 4700000 1600000 1600000 4700000 3300000"
chmod u+w "$sys/devices/system/cpu/cpu5/cpufreq/scaling_max_freq"
exits 0 free 1600
ceilings "${free}"

# A free the kernel refuses on one core still frees the others, fails,
# and keeps the hold's note, so another speed is still refused, and asking
# again frees the rest.
exits 0 hold 1600
chmod a-w "$sys/devices/system/cpu/cpu2/cpufreq/scaling_max_freq"
exits 1 free 1600
ceilings "4800000 4700000 1600000 3300000 4700000 3300000"
exits 75 hold 2000
chmod u+w "$sys/devices/system/cpu/cpu2/cpufreq/scaling_max_freq"
exits 0 free 1600
ceilings "${free}"
exits 0 hold 2000
ceilings "4800000 4700000 2000000 2000000 4700000 2000000"
exits 0 free 2000
ceilings "${free}"

# A hold or a free waits while another holds the lock, so two units
# started together take their turns rather than both passing the check.
mkdir -p "${EFFICIENT_CORES_RUN}"
exec 8>"${EFFICIENT_CORES_RUN}/lock"
flock 8
waits hold 1600
waits free
exec 8>&-
ceilings "${free}"

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
ceilings "${free}"

# A processor with no efficient cores holds nothing and fails, so a unit
# never reads as holding what it couldn't.
rm -r "$sys/devices/cpu_atom"
exits 69 hold 1600
ceilings "${free}"
