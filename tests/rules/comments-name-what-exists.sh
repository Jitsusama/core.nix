#!/usr/bin/env bash
# Holds code-health's stale mode over the tree given, against its waiting
# list, having first seen it refuse a planted tree three ways: a comment
# naming what isn't there, a waiting line nothing matches, and a waiver for
# a file that reads. So the check is seen to fail before it's trusted to
# pass, even once the list is empty. No file can be planted that the reader
# can't read, since each misreading found is mended; code-health's own
# tests hold what the gate says of one (gate.rs).
set -euo pipefail

tree=$1
waiting=$2

# The planted tree, with its lists beside it rather than in it, so the
# tree read holds only the one comment; gone however this ends.
scratch=$(mktemp -d)
trap 'rm -rf "${scratch}"' EXIT
planted=${scratch}/tree
mkdir -p "${planted}/modules/a"
printf '%s\n' '// Read with modules/a/gone.rs.' >"${planted}/modules/a/lib.rs"

# Runs stale mode on the planted tree against the list given, and requires
# it refuse (77), naming `expected` and not `unsaid`; what it said is kept
# to show only when it didn't, so a passing run's log holds no refusal.
refuses() {
  local list=$1 expected=$2 unsaid=$3 said status=0
  said=$(code-health stale "${planted}" "${list}" 2>&1) || status=$?
  if [[ ${status} -ne 77 ]]; then
    printf '%s\n' "${said}" >&2
    echo "code-health stale exited ${status} on a planted tree, not 77 (refused)" >&2
    exit 1
  fi
  if [[ ${said} != *"${expected}"* ]]; then
    printf '%s\n' "${said}" >&2
    echo "code-health stale refused a planted tree without naming ${expected}" >&2
    exit 1
  fi
  if [[ -n ${unsaid} && ${said} == *"${unsaid}"* ]]; then
    printf '%s\n' "${said}" >&2
    echo "code-health stale named ${unsaid}, which its waiting list holds" >&2
    exit 1
  fi
}

: >"${scratch}/none"
refuses "${scratch}/none" "modules/a/lib.rs:1: modules/a/gone.rs" ""
printf '%s\n' 'modules/a/lib.rs modules/a/gone.rs' 'modules/a/lib.rs modules/b' \
  >"${scratch}/more"
refuses "${scratch}/more" "modules/a/lib.rs modules/b (found nowhere" "modules/a/lib.rs:1:"
printf '%s\n' 'modules/a/lib.rs modules/a/gone.rs' 'modules/a/lib.rs unread' >"${scratch}/waived"
refuses "${scratch}/waived" "modules/a/lib.rs unread (read now: take it off)" "modules/a/lib.rs:1:"

code-health stale "${tree}" "${waiting}"
