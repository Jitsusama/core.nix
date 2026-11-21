# shellcheck shell=bash
# Says, in one line, how long the cores have spent in each rest state since
# boot, summed over every core, with each state's exit latency as the
# kernel has it now. While the hold on deep rest runs, the kernel never
# picks C6S or C10, so their totals stop growing: that shows the kernel
# kept to the hold, not what the chip itself did, which only its own
# counters say. It says there are none only where the kernel's cpuidle
# driver is none; a driver whose states can't be read, as a confinement
# hiding them would leave it, or a core whose states differ from the first
# core's in name or number, fails rather than say something wrong. A core
# with no cpuidle folder has nothing to add: one never brought up, or one
# whose driver let go of it as it went offline. intel_idle keeps a core's
# folder when it's taken offline after boot, so that core's totals, which
# stop there, are still added. The line counts the cores it adds.
# usage: rest-residency.sh [cpu folder]
# The cpu folder is /sys/devices/system/cpu unless a test gives its own.
set -euo pipefail
shopt -s nullglob

cpus=${1:-/sys/devices/system/cpu}
names=() latencies=() times=() entries=()

if [[ ! -r $cpus/cpuidle/current_driver ]]; then
  echo "The kernel's cpuidle driver can't be read in $cpus/cpuidle." >&2
  exit 1
fi
driver=$(<"$cpus/cpuidle/current_driver")
if [[ $driver == none ]]; then
  echo "The cores report no rest states, so there's nothing to add up."
  exit 0
fi

# By number rather than by glob, which would put state10 before state2.
for ((i = 0; ; i++)); do
  state=$cpus/cpu0/cpuidle/state$i
  [[ -r $state/name ]] || break
  names+=("$(<"$state/name")")
  latencies+=("$(<"$state/latency")")
  times+=(0)
  entries+=(0)
done

if ((${#names[@]} == 0)); then
  echo "The kernel's cpuidle driver is $driver, but the cores' rest states can't be read." >&2
  exit 1
fi

cores=0
for core in "$cpus"/cpu[0-9]*; do
  [[ -e $core/cpuidle ]] || continue
  cores=$((cores + 1))
  if [[ -e $core/cpuidle/state${#names[@]} ]]; then
    echo "${core##*/} has more rest states than cpu0, so they can't be added up." >&2
    exit 1
  fi
  for ((i = 0; i < ${#names[@]}; i++)); do
    state=$core/cpuidle/state$i
    if [[ ! -r $state/name ]]; then
      echo "${core##*/}'s rest state $i, cpu0's ${names[i]}, can't be read in $state." >&2
      exit 1
    fi
    name=$(<"$state/name")
    if [[ $name != "${names[i]}" ]]; then
      echo "${core##*/}'s rest state $i is $name, not ${names[i]}, so they can't be added up." >&2
      exit 1
    fi
    times[i]=$((times[i] + $(<"$state/time")))
    entries[i]=$((entries[i] + $(<"$state/usage")))
  done
done

line="At rest since boot, $cores cores:"
for ((i = 0; i < ${#names[@]}; i++)); do
  seconds=$((times[i] / 1000000))
  line+=" ${names[i]} (out in ${latencies[i]} us) $seconds s in ${entries[i]} entries;"
done
echo "${line%;}"
