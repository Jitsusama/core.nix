# shellcheck shell=bash
# Runs expect-test.qml once for each way a case can end, and fails unless
# qml exits as Expect.run says it does: 0 when every case holds, and 1 with
# a FAIL line naming what was thrown when one doesn't, whatever it was.
# A run that hangs is cut off, and fails too.
# usage: expect-test.sh <qtdeclarative> <folder holding expect-test.qml and Expect.js>
set -uo pipefail

qt=$1
folder=$2
failed=0

# Runs one ending, and checks its exit and what it said.
ends() {
  local ending=$1 meant=$2 said=$3 status=0 log
  log=$(QML_IMPORT_PATH=$qt/lib/qt-6/qml timeout 30 "$qt/bin/qml" \
    "$folder/expect-test.qml" -- "$ending" 2>&1) || status=$?
  if [ "$status" = 124 ]; then
    echo "FAIL a case that $ending: qml hung until cut off at 30 s; it said:"
    echo "$log"
    failed=1
  elif [ "$status" != "$meant" ]; then
    echo "FAIL a case that $ending: qml exited $status, meant $meant; it said:"
    echo "$log"
    failed=1
  elif [ -n "$said" ] && ! grep -qF "FAIL $said" <<<"$log"; then
    echo "FAIL a case that $ending: no 'FAIL $said' line; it said:"
    echo "$log"
    failed=1
  elif [ -z "$said" ] && grep -qF "FAIL" <<<"$log"; then
    echo "FAIL a case that $ending: it said FAIL, though every case held:"
    echo "$log"
    failed=1
  fi
}

ends holds 0 ""
ends fails 1 "a case that doesn't hold: said 1, meant 2"
ends null 1 "null"
ends word 1 "a bare word"
ends wordless 1 "RangeError"
ends unsayable 1 "a thrown object that can't be said"
exit "$failed"
