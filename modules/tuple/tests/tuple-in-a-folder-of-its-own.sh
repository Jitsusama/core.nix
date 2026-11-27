#!/usr/bin/env bash
# What the checks that run the real tuple share: a folder of their own,
# made the current one, holding Tuple's home, data and runtime folders, so
# a script run anywhere leaves nothing behind and its stop reaches only a
# daemon started there, never a person's. Sourced first, before any tuple
# command runs.

here=$(mktemp -d)
# The folders are its own before the trap is set, so the stop can only
# reach a daemon started here.
export HOME=$here XDG_DATA_HOME=$here/data XDG_RUNTIME_DIR=$here/run

# A trap's own failure would end a passing run as failed, so the stop and
# the removal each say what went wrong instead: a stop cut off may leave
# the daemon running outside a build, which takes its processes with it.
leave() {
  local stopped=0
  timeout -k 5 10 tuple stop > /dev/null 2>&1 || stopped=$?
  if [[ $stopped -eq 124 || $stopped -eq 137 ]]; then
    echo "tuple stop was cut off (status $stopped); its daemon may outlive $here" >&2
  fi
  rm -rf "$here" || echo "couldn't remove $here" >&2
}
trap leave EXIT
cd "$here" || exit
mkdir -m 700 run || exit
