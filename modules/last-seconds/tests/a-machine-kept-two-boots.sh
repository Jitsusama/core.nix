# shellcheck shell=bash
# Builds the rings last-seconds reads, for commands-agree to set its views
# at each width from: last-seconds-keep run over a stand-in machine for two
# boots, one stopped as at a shutdown and one cut short, as a reset leaves
# it. The stand-in draws, heats and waits, so every column has a figure.
# usage: a-machine-kept-two-boots.sh <folder to build it in>
set -euo pipefail

out=$1
machine=$out/machine
export LAST_SECONDS_STATE=$out/state
mkdir -p "$LAST_SECONDS_STATE"

put() {
  mkdir -p "$(dirname "$machine/$1")"
  printf '%s\n' "$2" >"$machine/$1"
}

put sys/class/powercap/intel-rapl:0/name package-0
put sys/class/powercap/intel-rapl:0/max_energy_range_uj 262143328850
put sys/class/powercap/intel-rapl:1/name psys
put sys/class/powercap/intel-rapl:1/max_energy_range_uj 262143328850
put sys/class/thermal/thermal_zone3/type x86_pkg_temp
put sys/class/thermal/thermal_zone3/temp 71000
put sys/class/hwmon/hwmon5/fan1_input 4200
put sys/class/hwmon/hwmon5/fan2_input 4150
put sys/class/power_supply/AC/type Mains
put sys/class/power_supply/AC/online 0
put sys/class/power_supply/BAT0/type Battery
put sys/class/power_supply/BAT0/voltage_now 16100000
put sys/class/power_supply/BAT0/current_now -2100000
put sys/class/power_supply/BAT0/capacity 63
put sys/class/power_supply/BAT0/status Discharging
for cpu in 0 1 2 3; do
  put "sys/devices/system/cpu/cpu$cpu/cpuidle/state0/name" POLL
  put "sys/devices/system/cpu/cpu$cpu/cpuidle/state1/name" C10
done
put proc/loadavg '6.31 4.02 2.20 9/1402 5120'
put proc/meminfo $'MemAvailable: 9123456 kB\nSwapTotal: 8388604 kB\nSwapFree: 8000000 kB'

# Each pass moves every counter on, as a second of work would: the
# energy, the waits and the time at rest.
step=0
counters() {
  step=$((step + 1))
  put sys/class/powercap/intel-rapl:0/energy_uj $((step * 280000))
  put sys/class/powercap/intel-rapl:1/energy_uj $((step * 410000))
  put proc/pressure/cpu "some avg10=0 avg60=0 avg300=0 total=$((step * 3000))"
  local full='full avg10=0 avg60=0 avg300=0 total=0'
  put proc/pressure/memory "some avg10=0 avg60=0 avg300=0 total=$((step * 900))"$'\n'"$full"
  put proc/pressure/io "some avg10=0 avg60=0 avg300=0 total=$((step * 1500))"$'\n'"$full"
  for cpu in 0 1 2 3; do
    put "sys/devices/system/cpu/cpu$cpu/cpuidle/state0/time" 0
    put "sys/devices/system/cpu/cpu$cpu/cpuidle/state1/time" $((step * 6000))
  done
}

# One boot: its id and start, then the recorder run a tenth of a second
# apart while the counters move, a pass at a time, so each record has
# grown from the last. It's stopped as its unit is at a shutdown, or
# killed, as a reset ends it, with nothing written on the way down.
boot() {
  local id=$1 began=$2 passes=$3 ends=$4
  put proc/sys/kernel/random/boot_id "$id"
  put proc/stat "btime $began"
  counters
  last-seconds-keep --root "$machine" --every 100 &
  local keeping=$!
  for _ in $(seq "$passes"); do
    # A pass a record: the recorder reads a tenth of a second apart.
    sleep 0.1
    counters
  done
  if [[ $ends == stopped ]]; then
    kill -TERM "$keeping"
    wait "$keeping"
  else
    kill -KILL "$keeping"
    wait "$keeping" || true
  fi
}

boot 53cb7920-3768-4896-bc1e-3870381c2398 1795100000 12 stopped
boot 949a397c-eafb-4b78-9dd6-b3d0e50c92e9 1795200000 30 cut
