#!/usr/bin/env bash
# Reads what the shell said running tests/displays.qml with reports off, given as $1: a share
# the windows daemon refused, asked with no : line to take its why, says the daemon's code and
# how many letters its words held, not the screen they name (Y2YBTS, 2026-11-20, 41ca61c4).
set -euo pipefail
said=$1
failed=0

# A line ending in what's wanted, so one saying more after the count doesn't pass for it.
wanted="launcher: brightness 33 refused: unread, 25 letters"
if ! WANTED=$wanted awk 'BEGIN { w = ENVIRON["WANTED"] }
  substr($0, length($0) - length(w) + 1) == w { found = 1 }
  END { exit !found }' "$said"; then
  echo "a refused share isn't said by its code and how many letters, wanted: $wanted"
  failed=1
fi

if grep -a 'launcher:' "$said" | grep -aF 'Harbour'; then
  echo "a line says the daemon's words with reports off"
  failed=1
fi

if [ "$failed" -ne 0 ]; then
  echo "what the launcher said:"
  grep -a 'launcher:' "$said" || true
fi
exit "$failed"
