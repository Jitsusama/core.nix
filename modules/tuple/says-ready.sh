# shellcheck shell=bash
# Says Tuple's daemon is ready, in the words modules/health reads as a
# daemon's ready line, once it can take a call: Tuple says only that it
# started, so a switch's restart of tuple.service was timed only to its
# start, not to the daemon serving again.
#
# It's ready once its signaller, which calls come through, has connected,
# which its log says; the time runs from the log's first timed line, the
# daemon beginning, to that one, both on the log's own clock. The log opens
# with "launching from pid", which has no time, and is new each start, the
# last kept as log.1.txt before it (home.nix's keepsLogs). A time past the
# wait, as a slow start or a clock stepped either way gives, says only that
# the signaller connected, and when. A wait that runs out says so, and why:
# the signaller not connecting, or systemd never saying the unit was active,
# which on a machine means its asks fail, since a unit that stops takes the
# waiter with it. Each is in words health doesn't read as a ready line, so a
# Tuple whose log stops saying it the same way, or a waiter that can't ask,
# shows in the journal rather than only going quiet. The wait is counted on
# the time since boot, which a step of the wall clock doesn't move.
#
# It runs as the unit's ExecStartPost, which systemd finishes before it
# writes "Started", and health takes a ready line only after that line. So
# it leaves a waiter in the unit and returns at once, and the waiter says it
# once the unit is active too, on the unit's own output, which the journal
# files under the unit and the name the unit gives it. Nothing it does can
# fail the unit: signed out or unreachable, the daemon never connects, and
# it says only that. When `tuple start` forked no daemon, as it doesn't when
# one already runs outside the unit, systemd takes the waiter, alone in the
# unit, for its main process, and the waiter would be all that kept it up:
# so named, it says so and leaves at once, and the unit stops as it did
# before there was a waiter. As the main process, how it ends is how the unit ends, so it
# always ends well, or Restart= would start Tuple again.
#
# Given systemctl, the unit's name, the daemon's log and how many seconds to
# wait.
set -uo pipefail

systemctl=$1
unit=$2
log=$3
wait=$4

# A line of Tuple's log with a time, which it leads with.
stamped='^[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{3}\|'
# One whole: its time, pid, thread and level, the place in its source
# sometimes, a file and line, and what it says, so a caller's name in what
# a line says can't pose as another.
timed="${stamped}[0-9]+\\|[0-9]+\\|[a-z()_]+\\|([A-Za-z0-9_.]+:[0-9]+\\|)?"

# A log line's time, HH:MM:SS.mmm, in milliseconds since midnight.
millis() {
  local hours=${1:0:2} minutes=${1:3:2} seconds=${1:6:2} parts=${1:9:3}
  echo $(((10#${hours} * 3600 + 10#${minutes} * 60 + 10#${seconds}) * 1000 + 10#${parts}))
}

# Sets `now` to the hundredths of a second since boot, forking nothing.
uptime() {
  local up
  read -r up _ </proc/uptime
  now=$((10#${up/./}))
}

waits() {
  local shown key value state main active="" first connected began ended took
  local now deadline
  uptime
  deadline=$((now + wait * 100))
  while ((now < deadline)); do
    if [[ -z ${active} ]]; then
      shown=$(timeout 2 "${systemctl}" --user show --property=ActiveState,MainPID "${unit}") ||
        shown=""
      state="" main=""
      while IFS='=' read -r key value; do
        case ${key} in
        ActiveState) state=${value} ;;
        MainPID) main=${value} ;;
        *) ;;
        esac
      done <<<"${shown}"
      # The unit leaves active only by stopping, which ends this with it.
      if [[ ${state} == active ]]; then
        # A main process of 0 is a start that left daemons systemd couldn't
        # choose between, which serve all the same.
        if [[ ${main} == "${BASHPID}" ]]; then
          echo "tuple: its start left no daemon of its own, so none to time"
          return 0
        fi
        active=yes
      fi
    fi
    if [[ -n ${active} ]] &&
      connected=$(grep -m1 -E "${timed}signaler state changed: [a-z]+ -> connected$" "${log}"); then
      first=$(grep -m1 -E "${stamped}" "${log}") || return 0
      began=$(millis "${first}")
      ended=$(millis "${connected}")
      took=$((ended - began))
      # A start that ran past midnight reads its end on the next day.
      ((took < 0)) && took=$((took + 86400000))
      # The wait runs from after `tuple start`, so a start slower than that
      # can connect within the wait and still read past it.
      if ((took > wait * 1000)); then
        echo "tuple: its signaller connected ${took} ms after its log began," \
          "past the ${wait} s it times"
        return 0
      fi
      echo "tuple: ready in ${took} ms"
      return 0
    fi
    # Neither systemd nor the log says when it changes, so it looks again
    # a tenth of a second on: the line lands that close behind the daemon.
    sleep 0.1
    uptime
  done
  if [[ -n ${active} ]]; then
    echo "tuple: its signaller hadn't connected ${wait} s after it started"
  else
    echo "tuple: systemd hadn't said it was active ${wait} s after it started"
  fi
  return 0
}

# However waits ends, even on a fault set -u stops it for, the waiter ends
# well: as the unit's main process, how it ends is how the unit does.
{
  trap 'exit 0' EXIT
  waits
} </dev/null 2>/dev/null &
