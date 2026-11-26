#!/usr/bin/env bash
# Runs the pattern a Mac's house-report-key job tests a key's access list
# by over sample lines as ls -le prints them: each of LEAKING must be
# taken as letting another account know or change the key, and none of
# LESSER.
#
# Usage: lists-test.sh PATTERN LEAKING LESSER, the last two files of one
# line each.
set -eu

pattern=$1

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
