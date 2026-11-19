# shellcheck shell=bash
# Proves efficient-cores.sh holds only the efficient cores' clock ceiling,
# within each core's own range, and frees it to each core's full clock.
# Usage: efficient-cores-test <efficient-cores>, run in an empty folder,
# where it builds a stand-in sysfs.
set -euo pipefail
cores=$1
sys=$PWD/sys
export EFFICIENT_CORES_SYSFS=$PWD

# core <n> <lowest kHz> <highest kHz>: a core's cpufreq, its ceiling at its
# highest, as the kernel boots it.
core() {
  local freq=$sys/devices/system/cpu/cpu$1/cpufreq
  mkdir -p "${freq}"
  echo "$2" >"${freq}/cpuinfo_min_freq"
  echo "$2" >"${freq}/scaling_min_freq"
  echo "$3" >"${freq}/cpuinfo_max_freq"
  echo "$3" >"${freq}/scaling_max_freq"
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
# between them, so the list's ranges are read rather than assumed.
core 0 400000 4800000
core 1 400000 4700000
core 2 400000 3700000
core 3 400000 3300000
core 4 400000 4700000
core 5 400000 3300000
mkdir -p "$sys/devices/cpu_atom"
echo "2-3,5" >"$sys/devices/cpu_atom/cpus"

"${cores}" hold 1600 >/dev/null
ceilings "4800000 4700000 1600000 1600000 4700000 1600000"
# Below a core's lowest clock it's held at its lowest, above its highest at
# its highest, so no ceiling is one the kernel would refuse.
"${cores}" hold 100 >/dev/null
ceilings "4800000 4700000 400000 400000 4700000 400000"
"${cores}" hold 9000 >/dev/null
ceilings "4800000 4700000 3700000 3300000 4700000 3300000"
"${cores}" hold 1600 >/dev/null
"${cores}" free >/dev/null
ceilings "4800000 4700000 3700000 3300000 4700000 3300000"

exits 0 --help
exits 64
exits 64 hold
exits 64 hold 16a
exits 64 hold 1600 more
exits 64 free 1600
exits 64 warm

# A processor with no efficient cores holds nothing, and says so.
rm -r "$sys/devices/cpu_atom"
said=$("${cores}" hold 1600)
ceilings "4800000 4700000 3700000 3300000 4700000 3300000"
if [[ "${said}" != *"no efficient cores"* ]]; then
  echo "with no efficient cores it said '${said}'"
  exit 1
fi
