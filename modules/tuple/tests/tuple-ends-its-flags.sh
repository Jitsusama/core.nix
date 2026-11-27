#!/usr/bin/env bash
# The launcher passes a contact to tuple after "--", so a handle that starts
# with a dash is taken as a name, never as a flag (TuplePeople.qml's act()).
# Each command it runs that way is asked here with one name too many, which
# tuple refuses by counting its names, so the probe gives it nothing it would
# act on; the build sandbox has no network and no account either, so even a
# tuple that went further could reach nobody. A dash-led name counted among
# the names shows "--" still ends tuple's flags. The same names without "--"
# must be refused as a flag; with one name too many there too, a tuple that
# stopped reading flags refuses them by count rather than going on.
set -euo pipefail

# Tuple's call helper says any tuple command may start Tuple's daemon, so
# each probe is held to a time and its output goes to a file rather than a
# pipe a daemon could keep open, and the daemon is stopped as the check ends.
trap 'timeout 10 tuple stop > /dev/null 2>&1 || true' EXIT

refuses() {
  local want=$1
  shift
  local said=$PWD/said
  # A refusal comes back at once; ten seconds is room for a loaded builder.
  if timeout 10 tuple "$@" > "$said" 2>&1 < /dev/null; then
    echo "tuple $* ran, where it should have refused its names by count" >&2
    exit 1
  else
    local status=$?
    if [[ $status -eq 124 ]]; then
      echo "tuple $* went on past ten seconds rather than refusing" >&2
      exit 1
    fi
  fi
  if [[ $(< "$said") != *"$want"* ]]; then
    echo "tuple $* said \"$(< "$said")\", not one naming \"$want\"" >&2
    exit 1
  fi
}

# "received 2" is how cobra's counts say two names came, whichever count a
# command keeps to.
for name in -dashed@example.com --dashed@example.com; do
  refuses "received 2" call start -- "$name" another
  refuses "received 2" call join -- "$name" another
  refuses "received 2" contacts favorite -- "$name" another
  refuses "received 2" contacts unfavorite -- "$name" another
done
refuses "unknown shorthand flag" call start -dashed@example.com another
refuses "unknown flag" call start --dashed@example.com another
