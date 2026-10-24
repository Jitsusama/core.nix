# shellcheck shell=bash
# Stands in for power-profiles-daemon's command: it lists the three profiles
# with balanced in use, and writes each profile it's asked to set to
# $STATE/calls, after the battery's state it was set in.
# usage: powerprofilesctl.sh list | powerprofilesctl.sh set <profile>
set -euo pipefail

# The list goes in one write, through cat, since bash's printf writes a line
# at a time: a reader that stops at the first line it wants, as grep -q does,
# would otherwise close the pipe on the rest.
profiles=$'  performance:\n* balanced:\n  power-saver:'

case "$1" in
  list) cat <<< "$profiles" ;;
  set) echo "$(cat "$STATE/now"): $2" >> "$STATE/calls" ;;
esac
