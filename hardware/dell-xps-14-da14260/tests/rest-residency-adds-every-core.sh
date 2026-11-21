# shellcheck shell=bash
# Checks rest-residency.sh adds each rest state over every core by its
# number, past state9 and cpu9, with each state's exit latency; says so
# plainly when the kernel has no cpuidle driver; and fails when the driver
# or its states can't be read, or a core's states differ from the first's
# in name or number; and leaves out a core with no cpuidle folder,
# counting the cores it adds.
# usage: rest-residency-adds-every-core.sh <rest-residency.sh>
set -euo pipefail

script=$1
cpus=$(mktemp -d)
failed=0

# A state of a core: its name, exit latency, time in microseconds and
# entries.
state() {
  mkdir -p "$cpus/$1/cpuidle/state$2"
  echo "$3" >"$cpus/$1/cpuidle/state$2/name"
  echo "$4" >"$cpus/$1/cpuidle/state$2/latency"
  echo "$5" >"$cpus/$1/cpuidle/state$2/time"
  echo "$6" >"$cpus/$1/cpuidle/state$2/usage"
}

expect() {
  local said
  said=$(bash "$script" "$cpus")
  if [[ $said != "$1" ]]; then
    echo "$2: it said \"$said\", not \"$1\"." >&2
    failed=1
  fi
}

refused() {
  if bash "$script" "$cpus" >/dev/null 2>&1; then
    echo "$1" >&2
    failed=1
  fi
}

refused "With no cpuidle folder to read, as a confinement hiding it would leave it, it didn't fail."

mkdir -p "$cpus/cpuidle"
echo none >"$cpus/cpuidle/current_driver"
expect "The cores report no rest states, so there's nothing to add up." \
  "With no cpuidle driver it didn't say so"

echo intel_idle >"$cpus/cpuidle/current_driver"
mkdir -p "$cpus/cpu0/cpuidle"
refused "With a driver but no states it could read, it didn't fail."

for core in cpu0 cpu1 cpu10; do
  for i in 0 1 2 3 4 5 6 7 8 9; do
    state "$core" "$i" "S$i" "$i" 0 0
  done
  state "$core" 10 C10 370 2000000 3
  state "$core" 2 C1E 10 1000000 5
done
mkdir -p "$cpus/cpu11"
expect "At rest since boot, 3 cores: S0 (out in 0 us) 0 s in 0 entries;\
 S1 (out in 1 us) 0 s in 0 entries; C1E (out in 10 us) 3 s in 15 entries;\
 S3 (out in 3 us) 0 s in 0 entries; S4 (out in 4 us) 0 s in 0 entries;\
 S5 (out in 5 us) 0 s in 0 entries; S6 (out in 6 us) 0 s in 0 entries;\
 S7 (out in 7 us) 0 s in 0 entries; S8 (out in 8 us) 0 s in 0 entries;\
 S9 (out in 9 us) 0 s in 0 entries; C10 (out in 370 us) 6 s in 9 entries" \
  "It didn't add each state over cpu0, cpu1 and cpu10 in its number's place"

mv "$cpus/cpu10/cpuidle/state10" "$cpus/state10"
refused "With cpu10 lacking a state cpu0 has, it didn't fail."
mv "$cpus/state10" "$cpus/cpu10/cpuidle/state10"

state cpu10 11 C11 400 0 0
refused "With cpu10 having a state cpu0 lacks, it didn't fail."
rm -r "$cpus/cpu10/cpuidle/state11"

echo C6S >"$cpus/cpu10/cpuidle/state2/name"
refused "With cpu10's third state not cpu0's, it didn't fail."

exit "$failed"
