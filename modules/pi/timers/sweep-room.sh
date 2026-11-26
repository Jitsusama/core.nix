# shellcheck shell=bash
# Waits, before the hourly sweep of ended sessions' work trees, for the
# machine to have room for it, run as the unit's ExecStartPre. The sweep
# walks thousands of trees, and on 26 November one run that started while
# the disk was already contended held the whole machine's I/O stalled a
# fifth of its time for half an hour. So it waits while the kernel's
# five-minute averages of memory and I/O stalls are high or under a
# quarter of memory is free. It never skips the hour: a host's own sweep
# takes over once the stamp of the last whole one is two hours old, on the
# host's one thread and with no limits, and two skipped hours on 26
# November did just that. So after 40 minutes it lets the sweep start
# however the machine stands, leaving the run its unit's own limits, and
# it lets it start at once on a reading it can't make. The marks are 70%
# of where an agent's heavy work stops: memory stalls past 5% or I/O stalls
# past 2% over a minute, or under 16 GB free on a 62 GiB machine.
#
#   pi-sweep-room [pressure folder] [meminfo] [tries] [seconds apart]
#
# The arguments are for its check; the unit gives none.

pressure=${1:-/proc/pressure}
meminfo=${2:-/proc/meminfo}
tries=${3:-80}
apart=${4:-30}

# The kernel's five-minute average of the share of time every task waited
# on `$1`, or nothing where it can't be read or isn't a number.
full() {
  awk '/^full/ { split($4, a, "="); if (a[2] ~ /^[0-9]+(\.[0-9]+)?$/) print a[2] }' \
    "$pressure/$1" 2> /dev/null || true
}

# What meminfo says of `$1` in kB, or nothing as above, or for none at all.
meminfo() {
  awk -v k="$1:" '$1 == k && $2 ~ /^[0-9]+$/ && $2 > 0 { print $2 }' "$meminfo" 2> /dev/null || true
}

# Whether `$1` is past `$2`, as decimals.
past() {
  awk -v a="$1" -v b="$2" 'BEGIN { exit !(a + 0 > b + 0) }'
}

tried=0
while :; do
  memory=$(full memory)
  io=$(full io)
  available=$(meminfo MemAvailable)
  total=$(meminfo MemTotal)
  if [ -z "$memory" ] || [ -z "$io" ] || [ -z "$available" ] || [ -z "$total" ]; then
    echo "pi-sweep starts: the machine's pressure or memory couldn't be read"
    exit 0
  fi
  free=$((available * 100 / total))
  said="memory stalled $memory%, I/O stalled $io% over five minutes, $free% of memory free"
  if ! past "$memory" 3.5 && ! past "$io" 1.4 && [ "$free" -ge 25 ]; then
    echo "pi-sweep starts: $said"
    exit 0
  fi
  if [ "$tried" -ge "$tries" ]; then
    echo "pi-sweep starts without room after waiting $((tries * apart / 60)) minutes: $said"
    exit 0
  fi
  if [ "$tried" -eq 0 ]; then
    echo "pi-sweep waits for room: $said (it starts at 3.5%, 1.4% and 25%)"
  fi
  tried=$((tried + 1))
  sleep "$apart"
done
