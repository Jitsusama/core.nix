# shellcheck shell=bash
# Shows that pi-fence's census, as core builds pi-fence, loads every
# package it imports and judges a machine: here one with no sessions,
# which holds. A census that can't load refuses with 126, saying why.
# Usage: census-loads.sh PI_FENCE
set -o errexit
set -o nounset
set -o pipefail

fence=$1
home=$(mktemp -d)
proc=$(mktemp -d)
status=0
said=$(HOME=$home "$fence" census --proc "$proc" 2>&1) || status=$?
if [ "$status" -ne 0 ]; then
  printf 'pi-fence census exited %s, saying:\n%s\n' "$status" "$said" >&2
  exit 1
fi
