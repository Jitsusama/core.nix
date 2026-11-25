#!/usr/bin/env bash
# The test of room.sh, which offers the Linux builder to the Mac's Nix only
# while the Mac has room: under the floor or swapping it empties the file
# Nix reads at once, and puts the line back only after the hold. sysctl and
# vm_stat are stand-ins that print what the test sets. Time passing is the
# job's own record of when the Mac was last short of room, set back.
#
#   room-test.sh ROOM
set -euo pipefail

room=$1
bin=${PWD}/bin
state=${PWD}/state
mkdir -p "${bin}" "${state}"
printf '#!%s\ncat %q\n' "${BASH}" "${PWD}/free" >"${bin}/sysctl"
cat >"${bin}/vm_stat" <<EOF
#!${BASH}
echo "Pages free:          1024."
echo "Swapouts:          \$(cat ${PWD}/swapouts)."
EOF
chmod +x "${bin}/sysctl" "${bin}/vm_stat"
echo "ssh-ng://linux-builder x86_64-linux,aarch64-linux /etc/nix/builder_ed25519 4 1 big-parallel -" \
  >"${PWD}/line"
offered=${PWD}/offered

# Looks once at the Mac given its share free and swap-outs so far, with 35%
# as the floor and 600 s as the hold, keeping what it said, how it ended
# and whether the file Nix reads holds the builder's line or nothing.
look() {
  echo "$1" >"${PWD}/free"
  echo "$2" >"${PWD}/swapouts"
  ended=0
  "${BASH}" "${room}" "${bin}/sysctl" "${bin}/vm_stat" "${PWD}/line" "${offered}" "${state}" 35 600 \
    2>"${PWD}/said" || ended=$?
  said=$(<"${PWD}/said")
  offer=something-else
  if cmp -s "${offered}" "${PWD}/line"; then
    offer=offered
  elif [[ -e ${offered} && ! -s ${offered} ]]; then
    offer=withheld
  fi
}
# Sets the last time the Mac was short of room this many seconds back.
pressed_ago() {
  echo "$((EPOCHSECONDS - $1))" >"${state}/pressed"
}
fail() {
  echo "room-test: $*; the file held ${offer}, and it said: ${said}" >&2
  exit 1
}

look 50 100
[[ ${offer} == withheld ]] || fail "a first look offered the builder before the hold ran"

pressed_ago 601
look 50 100
[[ ${offer} == offered ]] ||
  fail "a Mac at 50% free swapping nothing for the hold didn't get the builder's line back"
[[ ${said} == *"offered again"* ]] || fail "the builder offered again wasn't said"

look 50 100
[[ -z ${said} ]] || fail "an offer that didn't change was said again"

look 30 100
[[ ${offer} == withheld ]] || fail "a Mac at 30% free was still offered the builder"
[[ ${said} == *"30% free, under 35%"* ]] || fail "the builder withheld didn't say the Mac was under the floor"

pressed_ago 601
look 50 100
[[ ${offer} == offered ]] || fail "a Mac back above the floor for the hold didn't get the builder's line back"
look 50 105
[[ ${offer} == withheld ]] || fail "a Mac at 50% free that swapped pages out was still offered the builder"
[[ ${said} == *"5 pages swapped out"* ]] || fail "the builder withheld didn't say pages were swapped out"

pressed_ago 30
look 50 105
[[ ${offer} == withheld ]] || fail "a Mac that swapped out 30 s ago was offered the builder before the hold"

pressed_ago 590
look 50 105
[[ ${offer} == withheld ]] ||
  fail "a Mac short of room 590 s ago was offered the builder before the 600 s hold"

pressed_ago 601
look 50 105
[[ ${offer} == offered ]] || fail "a Mac that stopped swapping for the hold didn't get the builder back"

look "" 105
[[ ${offer} == withheld ]] || fail "a Mac whose room couldn't be read was still offered the builder"
[[ ${ended} == 1 ]] || fail "a room that couldn't be read ended ${ended}, not 1"
[[ ${said} == *"couldn't read the Mac's room"* ]] || fail "a room that couldn't be read wasn't said"

echo "room-test: the builder was offered only with room, and back only after the hold"
