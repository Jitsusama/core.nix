#!/usr/bin/env bash
# The test of mac-room.sh, which says whether a Mac has the memory to build
# its update in: room only from two readings a minute or more apart, both
# at the floor or over it, with nothing swapped out between; short from a
# first reading, one too soon, too stale or from later than now, one under
# the floor, any swap-out or a count started again; and can't tell, never
# room, from a figure it can't read, a record it can't keep, a floor that
# isn't a number or an argument left out. sysctl and
# vm_stat are stand-ins printing what the test sets, sysctl only for the
# figure mac-room.sh must ask for. Time passing is the record set back.
#
#   mac-room-test.sh MAC-ROOM
set -euo pipefail

room=$1
bin=${PWD}/bin
record=${PWD}/state/mac-room
mkdir -p "${bin}"
cat >"${bin}/sysctl" <<EOF
#!${BASH}
[ "\$*" = "-n kern.memorystatus_level" ] || { echo "sysctl: asked \$*" >&2; exit 1; }
[ "\$(cat ${PWD}/free)" != fails ] || exit 1
cat ${PWD}/free
EOF
cat >"${bin}/vm_stat" <<EOF
#!${BASH}
[ "\$(cat ${PWD}/swapouts)" != fails ] || exit 1
echo "Pages free:                               1024."
echo "Swapouts:                            \$(cat ${PWD}/swapouts)."
EOF
chmod +x "${bin}/sysctl" "${bin}/vm_stat"

failures=0
# Looks once given the share free and swap-outs now, with 35% as the
# floor, and checks how it answered and that it said a line holding why.
# What it says of a reading's age isn't held, since a second may pass
# between setting the record and the look.
look() {
  local case=$1 free=$2 swapouts=$3 want=$4 says=$5 ended=0
  echo "${free}" >"${PWD}/free"
  echo "${swapouts}" >"${PWD}/swapouts"
  "${BASH}" "${room}" "${bin}/sysctl" "${bin}/vm_stat" "${record}" 35 2>"${PWD}/said" || ended=$?
  if [ "${ended}" != "${want}" ] || ! grep -qF -- "${says}" "${PWD}/said"; then
    echo "FAIL ${case}: ended ${ended}, wanted ${want} saying '${says}': $(<"${PWD}/said")" >&2
    failures=$((failures + 1))
  fi
}
# Sets the record to a reading this many seconds back.
reading() {
  mkdir -p "${record%/*}"
  echo "$((EPOCHSECONDS - $1)) $2 $3" >"${record}"
}

rm -rf "${record%/*}"
look "a first reading" 60 100 0 "one reading of the Mac's room so far"
# The reading the first look kept, set a minute back, is what the next
# look pairs with: the follower's second ask a minute on.
read -r read_at was seen <"${record}"
echo "$((read_at - 60)) ${was} ${seen}" >"${record}"
look "the next look pairs with the one it kept" 50 100 1 "60% then 50% free"
reading 55 40 100
look "readings 55 s apart" 40 100 1 "40% then 40% free"
reading 598 40 100
look "readings nearly 600 s apart" 40 100 1 "40% then 40% free"
reading 53 40 100
look "readings 53 s apart" 40 100 0 "under 55 s"
reading 60 40 100
look "two readings a minute apart with room" 40 100 1 "40% then 40% free"
reading 60 35 100
look "both at the floor" 35 100 1 "35% then 35% free"
reading 60 40 100
look "the second under the floor" 30 100 0 "40% then 30% free"
reading 60 30 100
look "the first under the floor" 40 100 0 "30% then 40% free"
reading 60 40 100
look "a swap-out between" 40 103 0 "3 pages swapped out in"
reading 60 40 100
look "a swap-out count started again" 40 7 0 "the swap-out count started again"
reading 10 40 100
read -r before _ <"${record}"
look "readings too close" 50 100 0 "s ago, under 55 s"
read -r after kept _ <"${record}"
if [ "${after}" != "${before}" ] || [ "${kept}" != 40 ]; then
  echo "FAIL readings too close: the earlier reading wasn't kept" >&2
  failures=$((failures + 1))
fi
reading 700 40 100
look "a reading too stale" 40 100 0 "s ago, past 600 s"
reading 60 40 100
look "after a stale one, the next pairs with it" 40 100 1 "40% then 40% free"
reading -100 40 100
look "a reading from later than now" 40 100 0 "from later than now"
echo "not a reading" >"${record}"
look "a record that isn't one" 40 100 0 "one reading of the Mac's room so far"
reading 60 40 100
look "a share it can't read" fails 100 69 "couldn't read the Mac's room: its share of memory free"
look "swap-outs it can't read" 40 fails 69 "its swap-outs"
look "a share that isn't a number" "40%" 100 69 "its share of memory free"
# A record it can't keep can't tell, never room.
rm -rf "${record%/*}"
mkdir -p "${record%/*}"
reading 60 40 100
chmod a-w "${record%/*}"
look "a record it can't keep" 40 100 69 ""
chmod u+w "${record%/*}"
# Nor from a floor that isn't a whole number, or an argument left out.
reading 60 40 100
ended=0
"${BASH}" "${room}" "${bin}/sysctl" "${bin}/vm_stat" "${record}" 3.5 2>"${PWD}/said" || ended=$?
if [ "${ended}" != 69 ] || ! grep -qF "isn't a whole number" "${PWD}/said"; then
  echo "FAIL a floor that isn't a whole number: ended ${ended}: $(<"${PWD}/said")" >&2
  failures=$((failures + 1))
fi
ended=0
"${BASH}" "${room}" "${bin}/sysctl" "${bin}/vm_stat" "${record}" 2>/dev/null || ended=$?
if [ "${ended}" != 69 ]; then
  echo "FAIL an argument left out: ended ${ended}, wanted 69" >&2
  failures=$((failures + 1))
fi

[ "${failures}" -eq 0 ] || exit 1
