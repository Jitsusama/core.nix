# shellcheck shell=bash
# Stands in for busctl asking UPower about the battery: each time it's asked
# whether the machine is on battery, it answers with the next of the states
# in $STATE/steps, "on-battery percent" a line, and the percentage is the
# one that state gave. Asked once more than there are states, it says so in
# $STATE/errors and fails.
# usage: busctl.sh <busctl's arguments>
set -euo pipefail

case "$*" in
  *OnBattery*)
    if [ ! -s "$STATE/steps" ]; then
      echo "asked once too often" >> "$STATE/errors"
      exit 1
    fi
    head -n 1 "$STATE/steps" > "$STATE/now"
    tail -n +2 "$STATE/steps" > "$STATE/rest"
    mv "$STATE/rest" "$STATE/steps"
    read -r battery _ < "$STATE/now"
    echo "b $battery"
    ;;
  *Percentage*)
    read -r _ percent < "$STATE/now"
    echo "d $percent.6"
    ;;
esac
