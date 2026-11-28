# shellcheck shell=bash
# Shows every-hardware-address-is-made-up.sh refuses a maker's address no
# one kept, and passes each kind of made-up one and a kept one in upper
# case, saying what it says in the file given third. Usage:
# every-hardware-address-is-made-up-test.sh CHECK KEPT SAID
set -euo pipefail

check=$1
kept=$2
said=$3
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/made-up"
# A maker's address, then made-up ones of each kind, seven octets, and a
# kept one in upper case.
echo '00:00:00:00:00:01' >"$work/made-up/copied.txt"
echo '02:00:00:00:a0:00 52:54:00:12:34:56 ff:ff:ff:ff:ff:ff 01:00:5e:00:00:fb' \
  '00:00:5E:00:53:01 00:00:00:00:00:00 00:00:00:00:00:01:01 00:1B:66:AA:BB:CC' \
  >"$work/made-up/made-up.txt"
if bash "$check" "$work/made-up" "$kept" 2>"$work/said"; then
  echo "It passed a maker's address no one kept." >&2
  exit 1
fi
if ! diff "$said" "$work/said"; then
  echo "It refused the copied address with other words than the ones above." >&2
  exit 1
fi
