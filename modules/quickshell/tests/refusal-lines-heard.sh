#!/usr/bin/env bash
# Reads what the shell said running tests/refusal-lines.qml with reports off, given as $1:
# each refusal's line says how many letters the daemon's words held, its codes whole, and no
# line says the network the words named (Y2YBTS, 2026-11-20, 41ca61c4). Every case is read,
# so one failing doesn't hide another.
set -euo pipefail
said=$1
failed=0

heard() {
  if ! grep -aqF -- "$2" "$said"; then
    echo "$1 isn't said by how many letters, wanted: $2"
    failed=1
  fi
}

heard "a shut act's refusal" "launcher: verb:wifi-forget refused: 29 letters"
heard "a withheld act's why" "launcher: verb:wifi-join withheld: 41 letters"
heard "what an act asks besides, refused" "launcher: verb:copy's also refused: 30 letters"
error='{"code":"busy","said":"21 letters","message":"19 letters"}'
heard "a daemon's error, its code whole" \
  "launcher: wifi's daemon didn't answer 4 letters typed in \"wifi\": $error"

if grep -a 'launcher:' "$said" | grep -aF 'Harbour'; then
  echo "a line says the daemon's words with reports off"
  failed=1
fi

if [ "$failed" -ne 0 ]; then
  echo "what the launcher said:"
  grep -a 'launcher:' "$said" || true
fi
exit "$failed"
