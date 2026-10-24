# shellcheck shell=bash
# Stands in for niri: it lists the outputs in $OUTPUTS, and writes each mode
# it's asked to set to $STATE/calls, after the battery's state it was set in.
# usage: niri.sh msg --json outputs | niri.sh msg output <name> mode <mode>
set -euo pipefail

case "$1 $2" in
  "msg --json") cat "$OUTPUTS" ;;
  "msg output") echo "$(cat "$STATE/now"): $3 $4 $5" >> "$STATE/calls" ;;
esac
