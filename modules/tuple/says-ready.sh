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
# wait, as a clock stepped back gives, says nothing rather than a wrong
# figure. A wait that runs out once the unit is active says so, in words
# health doesn't read as a ready line, so a Tuple whose log stops saying it
# the same way shows in the journal rather than only going quiet.
#
# It runs as the unit's ExecStartPost, which systemd finishes before it
# writes "Started", and health takes a ready line only after that line. So
# it leaves a waiter in the unit and returns at once, and the waiter says it
# once the unit is active too, on the unit's own output, which the journal
# files under the unit and the name the unit gives it. Nothing it does can
# fail the unit: signed out or unreachable, the daemon never connects, and
# it says only that. When `tuple start` forked no daemon, as it doesn't when
# one already runs outside the unit, systemd finds no main process, and the
# waiter would be all that kept the unit up: so it leaves at once.
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

waits() {
  local shown key value state main active="" first connected began ended took
  while ((SECONDS < wait)); do
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
        [[ ${main} != 0 ]] || return
        active=yes
      fi
    fi
    if [[ -n ${active} ]] &&
      connected=$(grep -m1 -E "${timed}signaler state changed: [a-z]+ -> connected$" "${log}"); then
      first=$(grep -m1 -E "${stamped}" "${log}") || return
      began=$(millis "${first}")
      ended=$(millis "${connected}")
      took=$((ended - began))
      # A start that ran past midnight reads its end on the next day.
      ((took < 0)) && took=$((took + 86400000))
      ((took <= wait * 1000)) || return
      echo "tuple: ready in ${took} ms"
      return
    fi
    # Neither systemd nor the log says when it changes, so it looks again
    # a tenth of a second on: the line lands that close behind the daemon.
    sleep 0.1
  done
  if [[ -n ${active} ]]; then
    echo "tuple: its signaller hadn't connected ${wait} s after it started"
  fi
}

waits </dev/null 2>/dev/null &
