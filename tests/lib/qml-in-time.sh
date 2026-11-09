# shellcheck shell=bash
# Runs a test that times itself in Qt's engine, in a fresh process each
# time, until one is fast enough. How fast the engine runs the same code
# differs from one process to the next by as much as half, while a slower
# search slows every process, so the fastest of five is what's judged. A
# test exits 2 when it was only too slow; any other failure stops at once.
# usage: qml-in-time.sh <qtdeclarative> <test.qml>
set -euo pipefail

qt=$1
test=$2
# How many processes it may take to be fast enough.
tries=5
# What a test exits with when it was only too slow.
too_slow=2

for run in $(seq "$tries"); do
  status=0
  QML_IMPORT_PATH=$qt/lib/qt-6/qml "$qt/bin/qml" "$test" || status=$?
  [ "$status" = 0 ] && exit 0
  [ "$status" = "$too_slow" ] || exit "$status"
  echo "too slow in run $run of $tries"
done
exit 1
