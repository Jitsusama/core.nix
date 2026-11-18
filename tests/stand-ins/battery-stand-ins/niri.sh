# shellcheck shell=bash
# Stands in for niri: it lists the outputs in $OUTPUTS, and writes each mode
# it's asked to set to $STATE/calls, after the battery's state it was set in.
# usage: niri.sh msg --json outputs | niri.sh msg output <name> mode <mode>
set -euo pipefail
: "${STATE:?run.sh gives the folder the states are in}"

case "$1 $2" in
  "msg --json") cat "${OUTPUTS:?the check gives the outputs to list}" ;;
  "msg output") echo "$(< "$STATE/now"): $3 $4 $5" >> "$STATE/calls" ;;
  # A call this doesn't know would be answered wrongly, so it fails the run.
  *)
    echo "niri.sh doesn't stand in for: $*" >> "$STATE/errors"
    exit 1
    ;;
esac
