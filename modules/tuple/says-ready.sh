# shellcheck shell=bash
# Says Tuple's daemon is ready, in the words modules/health reads as a
# daemon's ready line, so a switch's restart of tuple.service is timed
# rather than counted as one that never said: Tuple says only that it
# started, with no time.
#
# It runs as the unit's ExecStartPost, which systemd finishes before it
# writes "Started", and health takes a ready line only after that line. So
# it leaves a waiter in the unit and returns at once, and the waiter says it
# once the unit is active. The time runs to the unit going active from the
# later of its leaving inactive and its last leaving active: a restart after
# a crash never passes through inactive, so it's timed from the crash. Each
# is systemd's own stamp on the boot's clock, so a clock step can't skew it.
# Nothing it does can fail the unit: it says nothing rather than a wrong
# time.
#
# Given systemctl, systemd-cat, the unit's name and how many times to look,
# a twentieth of a second apart.
set -uo pipefail

systemctl=$1
journal=$2
unit=$3
looks=$4

said() {
  "${systemctl}" --user show --property="$1" --value "${unit}"
}

waits() {
  local began left ready
  began=$(said InactiveExitTimestampMonotonic) || return
  left=$(said ActiveExitTimestampMonotonic) || return
  ((left > began)) && began=${left}
  ((began > 0)) || return
  for ((look = 0; look < looks; look++)); do
    if [[ $(said ActiveState) == active ]]; then
      ready=$(said ActiveEnterTimestampMonotonic) || return
      ((ready >= began)) || return
      echo "tuple: ready in $(((ready - began) / 1000)) ms" |
        "${journal}" --identifier=tuple
      return
    fi
    sleep 0.05
  done
}

waits </dev/null >/dev/null 2>&1 &
