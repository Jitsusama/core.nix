# shellcheck shell=bash
# Whether the machine has room for the hourly sweep of ended sessions'
# work trees to start, read as the unit's ExecCondition: 0 lets it start,
# 1 skips that hour, saying why, which systemd logs as a skip rather than
# a failure. The sweep walks thousands of trees, and on 26 November one
# run that started while the disk was already contended held the whole
# machine's I/O at a fifth of its time stalled for half an hour. So it
# starts only when the last five minutes kept memory and I/O stalls low
# and a quarter of memory is free. The next hour asks again; a host's own
# sweep takes over once the stamp of the last whole one is two hours old.
# The files it reads may be named, for its check.

pressure=${SWEEP_ROOM_PRESSURE:-/proc/pressure}
meminfo=${SWEEP_ROOM_MEMINFO:-/proc/meminfo}

# The share of the last five minutes in which every task waited on `$1`,
# or nothing where it can't be read, which is said below.
full() {
  awk '/^full/ { split($4, a, "="); print a[2] }' "$pressure/$1" 2> /dev/null || true
}

# What /proc/meminfo says of `$1`, in kB, or nothing as above.
meminfo() {
  awk -v k="$1:" '$1 == k { print $2 }' "$meminfo" 2> /dev/null || true
}

# Whether `$1` is past `$2`, as decimals.
past() {
  awk -v a="$1" -v b="$2" 'BEGIN { exit !(a + 0 > b + 0) }'
}

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
if past "$memory" 3.5 || past "$io" 1.4 || [ "$free" -lt 25 ]; then
  echo "pi-sweep waits for the next hour: $said (it starts at 3.5%, 1.4% and 25%)"
  exit 1
fi
echo "pi-sweep starts: $said"
