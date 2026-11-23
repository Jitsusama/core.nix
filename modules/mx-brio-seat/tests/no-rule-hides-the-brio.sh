#!/usr/bin/env bash
# Proves a seat machine's udev rules hold no rule marking a camera relayed
# or taking the MX Brio's seat tag off, in either order on its line. So the
# search can't pass for want of a match, it must first find the rule in a
# machine that shares the Brio. It reads through the rules' links into the
# store, and a folder it can't read fails rather than passing.
#
# Usage: no-rule-hides-the-brio.sh SHARED_RULES_FOLDER SEAT_RULES_FOLDER
set -euo pipefail
export LC_ALL=C

# The rule files in a folder that hide a camera from the seat; grep's 1,
# no match, is an answer, and anything past it an error.
hiding() {
  local found=0
  grep -RlE -e 'CAMERA_RELAYED\}="1"' \
    -e '091d.*TAG-="uaccess"' -e 'TAG-="uaccess".*091d' "$1"/ || found=$?
  if [ "$found" -gt 1 ]; then
    echo "the udev rules in $1 couldn't be read" >&2
    exit 1
  fi
}

# Each search is its own assignment, so a folder it can't read ends the
# script under set -e; inside a test, the failure would read as no match.
shared=$(hiding "$1")
if [ -z "$shared" ]; then
  echo "the search finds no rule hiding the MX Brio even where the sharing's is"
  exit 1
fi
seats=$(hiding "$2")
if [ -n "$seats" ]; then
  echo "these udev rules keep the MX Brio from the seat: $seats"
  exit 1
fi
