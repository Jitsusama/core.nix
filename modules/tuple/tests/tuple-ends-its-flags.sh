#!/usr/bin/env bash
# The launcher passes a contact to tuple after "--", so a handle that starts
# with a dash is taken as a name, never as a flag (TuplePeople.qml). Each
# command it runs that way is asked here with one name too many, which tuple
# refuses by counting before it does anything, so nobody is called, joined
# or starred. A dash-led name counted among the names shows "--" still ends
# tuple's flags; the same name without "--" is refused as a flag, which shows
# the probe would tell the two apart.
set -euo pipefail

refuses() {
  local want=$1
  shift
  local said
  if said=$(tuple "$@" 2>&1); then
    echo "tuple $* ran, where it should have refused its names by count" >&2
    exit 1
  fi
  if [[ $said != *"$want"* ]]; then
    echo "tuple $* said \"$said\", not \"$want\"" >&2
    exit 1
  fi
}

counted="accepts 1 arg(s), received 2"
refuses "$counted" call start -- --a-dashed-name another
refuses "$counted" call join -- --a-dashed-name another
refuses "$counted" contacts favorite -- --a-dashed-name another
refuses "$counted" contacts unfavorite -- --a-dashed-name another
refuses "unknown flag: --a-dashed-name" call start --a-dashed-name
