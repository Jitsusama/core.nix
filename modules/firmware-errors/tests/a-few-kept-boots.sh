# shellcheck shell=bash
# Builds the kept boots firmware-errors reads, for commands-agree to set
# its list and one boot's facts at each width from: four boots, the newest
# reset by the controller, one that lost power, a cold boot whose error
# region was cut short and one whose reason the specification doesn't
# name. The health tables are laid out as src/phat.rs's tests lay them.
# Beside them it describes a Panther Lake processor, so Intel's cause is
# named whatever machine sets the views.
# usage: a-few-kept-boots.sh <folder to build it in>
set -euo pipefail

state=$1/state
mkdir -p "$1"
printf 'vendor_id\t: GenuineIntel\ncpu family\t: 6\nmodel\t\t: 204\n' >"$1/cpuinfo"

# Prints hex digits as the bytes they spell.
hex() {
  local digits=$1 i
  for ((i = 0; i < ${#digits}; i += 2)); do
    printf '%b' "\\x${digits:i:2}"
  done
}

# Prints a number as so many little-endian bytes.
bytes() {
  local number=$1 count=$2 i
  for ((i = 0; i < count; i++)); do
    hex "$(printf %02x $(((number >> (8 * i)) & 255)))"
  done
}

# A health table naming a reset's source and reason, with Intel's four
# words when a cause is given: the header, an empty version record, then
# the reset reason's health record, its device path and its data.
phat() {
  local source=$1 reason=$2 cause=${3:-} data=6 vendors=0
  if [[ -n $cause ]]; then
    data=42
    vendors=1
  fi
  local offset=46
  local health=$((offset + data))
  hex 50484154
  bytes $((36 + 8 + health)) 4
  head -c 28 /dev/zero
  hex 0000080001000000
  bytes 1 2
  bytes "$health" 2
  hex 01000002
  hex e24c017a63f2774bb88ae6336b782c14
  bytes "$offset" 4
  # VenHw(0), in UTF-16 as the firmware writes it.
  hex 560065006e00480077002800300029000000
  hex 07
  bytes "$source" 1
  hex 00
  bytes "$reason" 1
  bytes "$vendors" 2
  if [[ -n $cause ]]; then
    hex bf018680b244674d9b27cca2642285ad
    bytes 36 2
    bytes 1 2
    bytes "$cause" 4
    bytes 0 4
    bytes 1 4
    bytes $((0x10390000)) 4
  fi
}

# A boot's folder, begun the given seconds before the view is set.
boot() {
  local id=$1 ago=$2
  mkdir -p "$state/$id"
  echo "@NOW-$ago@" >"$state/$id/began.in"
}

boot 1a2b3c4d5e6f40718293a4b5c6d7e8f9 3600
phat 1 0 $((0x400)) >"$state/1a2b3c4d5e6f40718293a4b5c6d7e8f9/phat-table"
# An error region holding no entries.
head -c 20 /dev/zero >"$state/1a2b3c4d5e6f40718293a4b5c6d7e8f9/bert-region"

boot 2b3c4d5e6f708192a3b4c5d6e7f8091a 90000
phat 2 $((0x24)) >"$state/2b3c4d5e6f708192a3b4c5d6e7f8091a/phat-table"

boot 3c4d5e6f708192a3b4c5d6e7f8091a2b 200000
phat 2 1 >"$state/3c4d5e6f708192a3b4c5d6e7f8091a2b/phat-table"
# An error region claiming 100 bytes of entries and holding none.
{
  head -c 12 /dev/zero
  bytes 100 4
  head -c 4 /dev/zero
} >"$state/3c4d5e6f708192a3b4c5d6e7f8091a2b/bert-region"

boot 4d5e6f708192a3b4c5d6e7f8091a2b3c 400000
phat 1 $((0x26)) >"$state/4d5e6f708192a3b4c5d6e7f8091a2b3c/phat-table"
