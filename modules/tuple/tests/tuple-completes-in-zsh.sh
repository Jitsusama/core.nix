#!/usr/bin/env bash
# Runs tuple-completion.zsh, given first, over Tuple's own completion, given
# second, in the check's build. Tab runs the real tuple, which may start its
# daemon, so the dozen or so editors are held to five minutes together, room
# for a loaded builder, and stopping the daemon to ten seconds. Each editor
# is named as it's typed in, so a run cut off for time ends naming the one
# that hung.
set -euo pipefail

export HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_RUNTIME_DIR=$PWD/run
mkdir -m 700 run
ended=0
timeout -k 5 300 zsh -f "$1" "$2" || ended=$?
timeout -k 5 10 tuple stop > /dev/null 2>&1 || true
if [[ $ended -eq 124 || $ended -eq 137 ]]; then
  echo "zsh was cut off (status $ended) in the last editor it named" >&2
fi
exit "$ended"
