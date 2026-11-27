#!/usr/bin/env bash
# Runs tuple-completion.zsh, given second, over Tuple's own completion, given
# third, in the folder of its own tuple-in-a-folder-of-its-own.sh, given
# first, makes. Tab runs the real tuple, which may start its
# daemon, so the dozen or so editors are held to five minutes together, room
# for a loaded builder, and stopping the daemon to ten seconds. Each editor
# is named as it's typed in, so a run cut off for time ends naming the one
# that hung.
set -euo pipefail

completion=$2 tuples=$3
# The library is handed in by its store path; shellcheck reads it from here.
# shellcheck source=SCRIPTDIR/tuple-in-a-folder-of-its-own.sh
source "$1"
ended=0
timeout -k 5 300 zsh -f "$completion" "$tuples" || ended=$?
if [[ $ended -eq 124 || $ended -eq 137 ]]; then
  echo "zsh was cut off (status $ended) in the last editor it named" >&2
fi
exit "$ended"
