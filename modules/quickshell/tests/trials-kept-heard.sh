#!/usr/bin/env bash
# Reads what the shell said running tests/trials-kept.qml with reports off, given as $1: a
# slow key's line says how many letters were typed, a slow step's keeps its arrow, a slow
# key of the launcher's own, as page down, says it whole, and no line names the row the
# letters found. Every case is read, so one failing doesn't hide
# another.
set -euo pipefail
said=$1
failed=0

heard() {
  if ! grep -aqF -- "$2" "$said"; then
    echo "$1 isn't said as it should be, wanted: $2"
    failed=1
  fi
}

heard "a slow key" "launcher: 13 letters took 900 ms"
heard "a slow step, its arrow kept" "launcher: → 13 letters took 900 ms"
heard "a slow key of the launcher's own, said whole" 'launcher: "page down" took 900 ms'

if grep -aF 'Harbour' "$said"; then
  echo "a line names the row with reports off"
  failed=1
fi

if [ "$failed" -ne 0 ]; then
  echo "what the launcher said:"
  grep -a 'launcher:' "$said" || true
fi
exit "$failed"
