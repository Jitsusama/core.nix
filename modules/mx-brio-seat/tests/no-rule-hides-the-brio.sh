#!/usr/bin/env bash
# Proves a seat machine's udev rules hold no rule marking a camera relayed,
# however it's assigned, or taking the MX Brio's seat tag off as the
# sharing's rule does, in either order on its line. A rule replacing the
# Brio's tags whole, or naming it on another line, isn't searched for. So no
# search can pass for want of a match, each is first shown to find what it's
# for: the relayed mark and the tag taken off in the rule of a machine that
# shares the Brio, and the reversed order and a spaced, added mark, which
# that rule doesn't use, in lines written here. It reads through the rules'
# links into the store, and a folder it can't read fails rather than passing.
#
# Usage: no-rule-hides-the-brio.sh SHARED_RULES_FOLDER SEAT_RULES_FOLDER
set -euo pipefail
export LC_ALL=C

# Spaced or not, and an assignment final, added or plain; a test of the
# mark, == or !=, as the seat's camera watch makes, isn't one.
relayed='CAMERA_RELAYED\}[[:space:]]*[:+]?=[[:space:]]*"1"'
taken='TAG[[:space:]]*-=[[:space:]]*"uaccess"'
untagged="091d.*${taken}"
reversed="${taken}.*091d"

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
echo 'KERNEL=="video*", TAG -= "uaccess", ENV{ID_MODEL_ID}=="091d"' >"${written}/reversed.rules"
echo 'KERNEL=="video*", ENV{CAMERA_RELAYED} += "1"' >"${written}/added.rules"
for pattern in "${reversed}" "${relayed}"; do
  shown=$(hiding "${written}" "${pattern}")
  if [[ -z "${shown}" ]]; then
    echo "the search for ${pattern} finds nothing even in a line written for it"
    exit 1
  fi
done

seats=$(hiding "$2" "${relayed}" "${untagged}" "${reversed}")
if [[ -n "${seats}" ]]; then
  echo "these udev rules mark a camera relayed or take the MX Brio's seat tag off: ${seats}"
  exit 1
fi
