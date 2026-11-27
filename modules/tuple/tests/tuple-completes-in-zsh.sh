#!/usr/bin/env bash
# Runs tuple-completion.zsh, given first, over Tuple's own completion, given
# second, in a folder of its own. Tab runs the real tuple, which may start its
# daemon, so the dozen or so editors are held to five minutes together, room
# for a loaded builder, and stopping the daemon to ten seconds. Each editor
# is named as it's typed in, so a run cut off for time ends naming the one
# that hung.
set -euo pipefail

# Its own folder, so run anywhere it leaves nothing behind and its stop
# reaches only the daemon Tab may have started there, never a person's.
# The folders are its own before the trap is set, as in
# tuple-shares-through-the-portal.sh, whose leave this is.
here=$(mktemp -d)
export HOME=$here XDG_DATA_HOME=$here/data XDG_RUNTIME_DIR=$here/run
leave() {
  local stopped=0
  timeout -k 5 10 tuple stop > /dev/null 2>&1 || stopped=$?
  if [[ $stopped -eq 124 || $stopped -eq 137 ]]; then
    echo "tuple stop was cut off (status $stopped); its daemon may outlive $here" >&2
  fi
  rm -rf "$here" || echo "couldn't remove $here" >&2
}
trap leave EXIT
cd "$here"
mkdir -m 700 run
ended=0
timeout -k 5 300 zsh -f "$1" "$2" || ended=$?
if [[ $ended -eq 124 || $ended -eq 137 ]]; then
  echo "zsh was cut off (status $ended) in the last editor it named" >&2
fi
exit "$ended"
