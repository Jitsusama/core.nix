#!/usr/bin/env bash
# Reads what the shell said running tests/wifi-acts.qml with reports off, given as $1: a join
# wifid refused is said by its act and code, and no launcher line says the network's name, the
# passwords typed, or wifid's own words (Y2YBTS, 2026-11-20, 41ca61c4).
set -euo pipefail
said=$1
failed=0

wanted="launcher: verb:wifi-join refused: WrongPassword"
if ! grep -aqF -- "$wanted" "$said"; then
  echo "a refused join isn't said by its act and code, wanted: $wanted"
  failed=1
fi

for words in Lantern Campus horse staple "didn't work"; do
  if grep -a 'launcher:' "$said" | grep -aF -- "$words"; then
    echo "a line says \"$words\" with reports off"
    failed=1
  fi
done

if [ "$failed" -ne 0 ]; then
  echo "what the launcher said:"
  grep -a 'launcher:' "$said" || true
fi
exit "$failed"
