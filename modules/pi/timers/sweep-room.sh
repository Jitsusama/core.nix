# shellcheck shell=bash
# Waits, before the hourly sweep of ended sessions' work trees, for the
# machine to have room for it, run as the unit's ExecCondition. The sweep
# walks thousands of trees, and on 26 November one run that started while
# the disk was already contended held the whole machine's I/O stalled a
# fifth of its time for half an hour. So it waits while the kernel's
# five-minute averages of memory and I/O stalls are high or under a
# quarter of memory is free, and starts the sweep once they fall back.
# Those averages trail a stall that ends by minutes, so a wait often runs
# most of its length. A wait that runs out skips the hour, since a sweep
# started without room on 27 November read 367 MB/s for ten minutes on a
# disk already saturated. A skip must leave the next run its turn: a
# host's own sweep takes over once the last whole sweep ended two hours
# ago, on the host's one thread and with no limits, and two skipped hours
# on 26 November did just that. So past its wait it skips only when the
# stamp, at the next top of the hour, when the hourly timer runs it
# again, will still be young enough for that run to start before the
# hosts. That holds however this run began, by hand, late or after a
# suspend, since it judges by the clock as it ends. It lets the sweep
# start at once when the last sweep's stamp is too old for a wait to end
# before the hosts take over, is dated ahead of the clock, or on a reading
# it can't make. It reads the stamp again on every pass, the last too,
# since a machine that sleeps through a wait wakes with the stamp older
# than the wait counted,
# and sleeps between passes a second at a time by the wall clock, since
# `sleep` doesn't count a suspend, so a pass ends within a second of
# waking, and no longer than its count of seconds if the clock steps
# back. It ends a wait two minutes and one pass short of the hosts' two
# hours, so its last sleep and the sweep's own start to its lock end
# before them. The stall marks, 3.5% and 1.4%, are 70% of the
# 5% and 2% at which an agent's heavy work stops, held over five minutes
# as a start is judged; a quarter free is the jobs door's own room.
# Its help takes the shape every command of ours does (docs/commands.md),
# by hand.

# A condition that ends 1 to 254 skips the hour and leaves the unit
# unfailed, so a wait that fails itself, by a command or by bash, lets
# the sweep start rather than skip, as it did when this ran before it.
# Only a skip it chose, and being asked wrongly, end otherwise. Under a
# unit, which systemd marks with INVOCATION_ID, being asked wrongly ends
# 255, the one end that fails a condition, so a unit whose words went
# wrong fails where it's seen rather than skipping every hour unheard.
skipping=false
ended() {
  local status=$?
  if [ "$status" -eq 64 ]; then
    [ -z "${INVOCATION_ID:-}" ] || exit 255
    return
  fi
  "$skipping" || exit 0
}
trap ended EXIT

say_help() {
  cat << 'EOF'
pi-sweep-room  wait for room before the tree sweep

  Reads the kernel's memory and I/O stalls and free
  memory every APART seconds, and ends once the machine
  has room. After WAIT seconds without room it skips the
  hour when the next hour's run can still start before
  the hosts sweep, and else lets the sweep start. It
  ends at once when the last sweep, by its STAMP, ended
  too long ago to wait, and on a reading it can't make.
  The sweep's unit runs it before each hourly sweep.

EXAMPLES
  cd ~/.local/state/pi/agentic-harness.pi/work
  pi-sweep-room /proc/pressure /proc/meminfo 900 30 \
    sweep.lock.outside

EXIT
  0   the sweep may start
  1   the sweep skips this hour
  64  asked wrongly, or 255 under a unit
EOF
}

for word in "$@"; do
  case $word in
  -h | --help)
    say_help
    exit 0
    ;;
  *) ;;
  esac
done
if [ "$#" -ne 5 ]; then
  printf '%s\n' "pi-sweep-room takes PRESSURE MEMINFO WAIT APART STAMP" \
    "  try  pi-sweep-room --help" >&2
  exit 64
fi

pressure=$1
meminfo=$2
most=$3
apart=$4
stamp=$5

# The kernel's five-minute average of the share of time every task waited
# on `$1`, or nothing where it can't be read or isn't a number.
full() {
  awk '/^full/ { split($4, a, "="); if (a[2] ~ /^[0-9]+(\.[0-9]+)?$/) print a[2] }' \
    "$pressure/$1" 2> /dev/null || true
}

# What meminfo says of `$1` in kB, or nothing as above. MemAvailable may
# be 0, on a machine out of memory.
meminfo() {
  awk -v k="$1:" '$1 == k && $2 ~ /^[0-9]+$/ { print $2 }' "$meminfo" 2> /dev/null || true
}

# Whether `$1` is past `$2`, as decimals.
past() {
  awk -v a="$1" -v b="$2" 'BEGIN { exit !(a + 0 > b + 0) }'
}

# How many seconds ago the last whole sweep ended, by its stamp, or
# nothing when there's none, as before the first, when hosts sweep, or
# when it's dated ahead of the clock, which no wait can be judged by.
age() {
  local stamped now
  stamped=$(stat -c %Y "$stamp" 2> /dev/null || true)
  now=$(date +%s)
  case $stamped in
  '' | *[!0-9]*) ;;
  *) [ "$now" -lt "$stamped" ] || echo $((now - stamped)) ;;
  esac
}

# How many seconds are left to the next top of the hour, by the local
# clock the hourly timer keeps.
to_the_hour() {
  local minute second
  read -r minute second < <(date '+%M %S')
  echo $((3600 - 10#$minute * 60 - 10#$second))
}

# The oldest the stamp may be while a sweep still waits.
latest=$((7200 - apart - 120))
ago=$(age)
if [ -z "$ago" ]; then
  echo "pi-sweep starts: no sweep has left a stamp it can read, so the hosts sweep meanwhile"
  exit 0
fi
if [ "$ago" -gt $((latest - most)) ]; then
  echo "pi-sweep starts: the last sweep ended $((ago / 60)) minutes ago, too long to wait"
  exit 0
fi

SECONDS=0
waited=false
while :; do
  memory=$(full memory)
  io=$(full io)
  available=$(meminfo MemAvailable)
  total=$(meminfo MemTotal)
  [ "$total" != 0 ] || total=""
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
      echo "pi-sweep starts after waiting $SECONDS s for room: $said"
    else
      echo "pi-sweep starts: $said"
    fi
    exit 0
  fi
  # Read again before judging, since a sleep ages it, a suspend most.
  ago=$(age)
  if [ -z "$ago" ]; then
    echo "pi-sweep starts without room: the last sweep's stamp can't be read: $said"
    exit 0
  fi
  if [ "$ago" -ge "$latest" ]; then
    echo "pi-sweep starts without room: the last sweep ended $((ago / 60)) minutes ago," \
      "too long to wait more: $said"
    exit 0
  fi
  if [ "$SECONDS" -ge "$most" ]; then
    # As old as the next hour's run, at the next top of the hour, finds it.
    if [ $((ago + $(to_the_hour))) -lt "$latest" ]; then
      echo "pi-sweep skips this hour without room after waiting $SECONDS s: $said"
      skipping=true
      exit 1
    fi
    echo "pi-sweep starts without room after waiting $SECONDS s: $said"
    exit 0
  fi
  if ! "$waited"; then
    echo "pi-sweep waits for room: $said (it starts at 3.5%, 1.4% and 25%)"
    waited=true
  fi
  # Counted too, so a clock stepped back ends the pass on time.
  slept=$SECONDS
  sleeps=0
  until [ $((SECONDS - slept)) -ge "$apart" ] || [ "$sleeps" -ge "$apart" ]; do
    sleep 1
    sleeps=$((sleeps + 1))
  done
done
