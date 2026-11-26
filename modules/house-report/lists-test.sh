#!/usr/bin/env bash
# Runs the pattern a Mac's house-report-key job tests a key's access list
# by over sample lines as ls -le prints them: each of LEAKING must be
# taken as letting another account know or change the key, and none of
# LESSER.
#
# Usage: lists-test.sh PATTERN LEAKING LESSER, the last two files holding
# one sample line per line.
set -eu

pattern=$1

# An empty pattern, as checks.nix reads one when the job's line has
# changed shape, takes every line, and would fail on a lesser sample as
# though the pattern were too broad (keys said plainly round 5's F3).
if [[ -z ${pattern} ]]; then
  echo "no pattern was read from the job" >&2
  exit 1
fi

while IFS= read -r line; do
  if ! grep -qE -- "${pattern}" <<<"${line}"; then
    echo "not taken as known: ${line}" >&2
    exit 1
  fi
done <"$2"

while IFS= read -r line; do
  if grep -qE -- "${pattern}" <<<"${line}"; then
    echo "taken as known: ${line}" >&2
    exit 1
  fi
done <"$3"
