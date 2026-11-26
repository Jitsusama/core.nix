# shellcheck shell=bash
# Waits, before the hourly sweep of ended sessions' work trees, for the
# machine to have room for it, run as the unit's ExecStartPre. The sweep
# walks thousands of trees, and on 26 November one run that started while
# the disk was already contended held the whole machine's I/O stalled a
# fifth of its time for half an hour. So it waits while the kernel's
# five-minute averages of memory and I/O stalls are high or under a
# quarter of memory is free, and starts the sweep as soon as they aren't.
# It never skips the hour: a host's own sweep takes over once the last
# whole sweep ended two hours ago, on the host's one thread and with no
# limits, and two skipped hours on 26 November did just that. So past its
# wait it lets the sweep start however the machine stands, and it lets it
# start at once on a reading it can't make. The marks are 70% of where an
# agent's heavy work stops: memory stalls past 5% or I/O stalls past 2%
# over a minute, or under 16 GB free on a 62 GiB machine.
#
#   pi-sweep-room <pressure folder> <meminfo> <seconds to wait> <seconds apart>

pressure=$1
meminfo=$2
most=$3
apart=$4

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

SECONDS=0
waited=false
while :; do
  memory=$(full memory)
  io=$(full io)
  available=$(meminfo MemAvailable)
  total=$(meminfo MemTotal)
  unread=""
  [ -n "$memory" ] || unread="$unread memory pressure,"
  [ -n "$io" ] || unread="$unread I/O pressure,"
  [ -n "$available" ] || unread="$unread available memory,"
  [ -n "$total" ] || unread="$unread memory in all,"
  if [ -n "$unread" ]; then
    echo "pi-sweep starts: the machine's${unread%,} couldn't be read"
    exit 0
  fi
  free=$((available * 100 / total))
  said="memory stalled $memory%, I/O stalled $io% over five minutes, $free% of memory free"
  if ! past "$memory" 3.5 && ! past "$io" 1.4 && [ "$free" -ge 25 ]; then
    if "$waited"; then
      echo "pi-sweep starts after waiting $((SECONDS / 60)) minutes for room: $said"
    else
      echo "pi-sweep starts: $said"
    fi
    exit 0
  fi
  if [ "$SECONDS" -ge "$most" ]; then
    echo "pi-sweep starts without room after waiting $((SECONDS / 60)) minutes: $said"
    exit 0
  fi
  if ! "$waited"; then
    echo "pi-sweep waits for room: $said (it starts at 3.5%, 1.4% and 25%)"
    waited=true
  fi
  sleep "$apart"
done
