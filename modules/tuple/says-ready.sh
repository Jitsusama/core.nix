# shellcheck shell=bash
# Says Tuple's daemon is ready, run once tuple start has returned with it
# running, in the words modules/health reads as a daemon's ready line, so a
# switch's restart of tuple.service is timed to here rather than counted
# as one that never said. The time is from systemd starting the unit, its
# log keeping included, to now.
#
# Given systemctl and the unit's name.
set -euo pipefail

systemctl=$1
unit=$2

began=$("${systemctl}" --user show --property=InactiveExitTimestamp --value \
  --timestamp=us+utc "${unit}")
start=$(date --date="${began}" +%s%6N)
now=$(date +%s%6N)
echo "tuple: ready in $(((now - start) / 1000)) ms"
