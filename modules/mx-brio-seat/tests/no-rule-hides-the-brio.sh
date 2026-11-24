#!/usr/bin/env bash
# Proves a seat machine's udev rules hold no rule marking a camera relayed
# or taking the MX Brio's seat tag off, in either order on its line. So no
# search can pass for want of a match, each is first shown to find what it's
# for: the relayed mark and the tag taken off in the rule of a machine that
# shares the Brio, and the reversed order, which that rule doesn't use, in a
# line written here. It reads through the rules' links into the store, and a
# folder it can't read fails rather than passing.
#
# Usage: no-rule-hides-the-brio.sh SHARED_RULES_FOLDER SEAT_RULES_FOLDER
set -euo pipefail
export LC_ALL=C

relayed='CAMERA_RELAYED\}="1"'
untagged='091d.*TAG-="uaccess"'
reversed='TAG-="uaccess".*091d'

# The rule files in a folder holding a line one of the patterns after it
# matches; grep's 1, no match, is an answer, and anything past it an error.
hiding() {
  local folder=$1 found=0 pattern patterns=()
  shift
  for pattern in "$@"; do
    patterns+=(-e "${pattern}")
  done
  grep -RlE "${patterns[@]}" "${folder}"/ || found=$?
  if [[ "${found}" -gt 1 ]]; then
    echo "the udev rules in ${folder} couldn't be read" >&2
    exit 1
  fi
}

# Each search is its own assignment, so a folder it can't read ends the
# script under set -e; inside a test, the failure would read as no match.
for pattern in "${relayed}" "${untagged}"; do
  shared=$(hiding "$1" "${pattern}")
  if [[ -z "${shared}" ]]; then
    echo "the search for ${pattern} finds nothing even in the sharing's rules"
    exit 1
  fi
done
written=$(mktemp -d)
trap 'rm -rf "${written}"' EXIT
echo 'KERNEL=="video*", TAG-="uaccess", ENV{ID_MODEL_ID}=="091d"' >"${written}/reversed.rules"
shown=$(hiding "${written}" "${reversed}")
if [[ -z "${shown}" ]]; then
  echo "the search for ${reversed} finds nothing even in a line written for it"
  exit 1
fi

seats=$(hiding "$2" "${relayed}" "${untagged}" "${reversed}")
if [[ -n "${seats}" ]]; then
  echo "these udev rules mark a camera relayed or take the MX Brio's seat tag off: ${seats}"
  exit 1
fi
