# shellcheck shell=bash
# Runs every QML case in the folder it starts in, each for at most a minute,
# and names each that fails or runs past its minute, since the build's log
# otherwise says only the exit code of the first, not which case, nor any
# after it.
# usage: runs-every-case.sh <qml>
set -uo pipefail

qml=$1
failed=0
for case in *.qml; do
  timeout 60 "$qml" "$case" || {
    code=$?
    if [ "$code" = 124 ]; then
      echo "$case ran past its 60 s" >&2
    else
      echo "$case failed, exit code $code" >&2
    fi
    failed=1
  }
done
exit "$failed"
