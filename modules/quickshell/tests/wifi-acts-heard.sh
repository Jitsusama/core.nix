#!/usr/bin/env bash
# Reads what the shell said running tests/wifi-acts.qml with reports off, given as $1: a join
# wifid refused is said by its act and code, and no launcher line says the network's name, the
# passwords or an account's answers typed, or wifid's own words (Y2YBTS, 2026-11-20, 41ca61c4).
# wifid keys a join as verb:wifi-join whatever the network, so its line reads the same with
# reports on: this guards that nothing typed into a join reaches the journal, not how the two
# modes differ: a refusal with reports off is refusal-lines-heard.sh's, and with them on
# refusalShown's in pacing.qml.
set -euo pipefail
said=$1
failed=0

# A line ending in what's wanted, so one saying more after the code doesn't pass for it.
wanted="launcher: verb:wifi-join refused: WrongPassword"
if ! WANTED=$wanted awk 'BEGIN { w = ENVIRON["WANTED"] }
  substr($0, length($0) - length(w) + 1) == w { found = 1 }
  END { exit !found }' "$said"; then
  echo "a refused join isn't said by its act and code, wanted: $wanted"
  failed=1
fi

# The networks, both passwords, wifid's words and each answer an account's join was given.
for words in Lantern Campus horse staple "didn't work" campus.example.org analytical; do
  if grep -a 'launcher:' "$said" | grep -aF -- "$words"; then
    echo "a line says \"$words\" with reports off"
    failed=1
  fi
done

# The account's name, as a word of its own, since its letters fall inside others.
if grep -a 'launcher:' "$said" | grep -awF -- ada; then
  echo "a line says the account's name with reports off"
  failed=1
fi

if [ "$failed" -ne 0 ]; then
  echo "what the launcher said:"
  grep -a 'launcher:' "$said" || true
fi
exit "$failed"
