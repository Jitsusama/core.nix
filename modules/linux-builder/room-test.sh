#!/usr/bin/env bash
# The test of room.sh, which offers the Linux builder to the Mac's Nix only
# while the Mac has room: under the floor, swapping, unread or with its own
# record gone or from later than now it holds no builder at once, and
# gives the line back only after the hold, at its edge; a step that fails
# leaves no builder offered; each change is said once, with when. sysctl and vm_stat
# are stand-ins that print what the test sets, sysctl only for the figure
# room.sh must ask for, and either may fail. Time passing is the job's own
# record of when the Mac was last short of room, set back.
#
#   room-test.sh ROOM
set -euo pipefail

room=$1
bin=${PWD}/bin
state=${PWD}/state
mkdir -p "${bin}" "${state}"
cat >"${bin}/sysctl" <<EOF
#!${BASH}
[ "\$*" = "-n kern.memorystatus_level" ] || { echo "sysctl: asked \$*" >&2; exit 1; }
[ "\$(cat ${PWD}/free)" != fails ] || exit 1
cat ${PWD}/free
EOF
cat >"${bin}/vm_stat" <<EOF
#!${BASH}
[ "\$(cat ${PWD}/swapouts)" != fails ] || exit 1
echo "Pages free:          1024."
echo "Swapouts:          \$(cat ${PWD}/swapouts)."
EOF
chmod +x "${bin}/sysctl" "${bin}/vm_stat"
key=/etc/nix/builder_ed25519
echo "ssh-ng://linux-builder x86_64-linux,aarch64-linux ${key} 4 1 big-parallel -" >"${PWD}/line"
offered=${PWD}/offered

# Looks once at the Mac given its share free and swap-outs so far, with 35%
# as the floor and 600 s as the hold, keeping what it said, how it ended
# and whether the file Nix reads holds the builder's line, or no builder
# with only comments saying why and where its log says when.
look() {
  echo "$1" >"${PWD}/free"
  echo "$2" >"${PWD}/swapouts"
  ended=0
  "${BASH}" "${room}" "${bin}/sysctl" "${bin}/vm_stat" "${PWD}/line" "${offered}" "${state}" \
    35 600 2>"${PWD}/said" || ended=$?
  said=$(<"${PWD}/said")
  offer=something-else
  if cmp -s "${offered}" "${PWD}/line"; then
    offer=offered
  elif [[ -s ${offered} ]] && ! grep -qv '^# ' "${offered}" &&
    head -n 1 "${offered}" | grep -qx '# withheld by linux-builder-room: ..*' &&
    grep -qxF "# linux-builder-room's log says when, as launchd's job names it" "${offered}"; then
    offer=withheld
  fi
}
# Sets the last time the Mac was short of room this many seconds back.
pressed_ago() {
  echo "$((EPOCHSECONDS - $1))" >"${state}/pressed"
}
fail() {
  echo "room-test: $*; the file held ${offer}, it ended ${ended} and said: ${said}" >&2
  exit 1
}
# Fails saying $3 unless the look left the file $1 and said what $2 matches,
# in one line at most, so a change said twice in a look fails too.
holds() {
  [[ ${offer} == "$1" ]] || fail "$3"
  [[ ${said} != *$'\n'* ]] || fail "$3, or said it more than once"
  # shellcheck disable=SC2053 # $2 is a pattern, matched as one.
  [[ ${said} == $2 ]] || fail "$3, or didn't say so as it should"
  [[ -z ${said} || ${said} =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z\ room:\  ]] ||
    fail "$3, or said it without when"
  [[ ${ended} == 0 ]] || fail "$3, ending ${ended}"
}

look 50 100
holds withheld "*isn't offered: it had no record of the Mac's room yet" \
  "a first look offered the builder before the hold ran"

pressed_ago 600
look 50 100
holds offered "*offered again*" \
  "a Mac at 50% free swapping nothing for exactly the hold wasn't offered"
look 50 100
holds offered "" "an offer that didn't change was said again"

pressed_ago 601
look 34 100
holds withheld "*isn't offered: 34% free, under 35%" \
  "a Mac at 34% free was still offered the builder"
look 35 100
holds withheld "" "a Mac short of room a look ago was offered before the hold, or said again"

pressed_ago 601
look 35 100
holds offered "*offered again*" "a Mac at exactly 35% free for the hold wasn't offered"
look 50 105
holds withheld "*isn't offered: 5 pages swapped out" \
  "a Mac at 50% free that swapped out was still offered"
look 50 105
holds withheld "" "a Mac that swapped out a look ago was offered before the hold"
pressed_ago 30
look 50 105
holds withheld "" "a Mac that swapped out 30 s ago was offered before the hold"
# 595 s rather than 599 s, since the test and room.sh each read the clock
# and a second may tick between; 600 s offering above holds the edge.
pressed_ago 595
look 50 105
holds withheld "" "a Mac short of room 595 s ago was offered before the 600 s hold"

pressed_ago 601
look 50 105
holds offered "*offered again*" "a Mac quiet for the hold after swapping wasn't offered"
look 50 3
holds withheld "*isn't offered: the swap-out count started again*" \
  "a swap-out count that started again, as after a restart, wasn't taken as swapping"
look 50 3
holds withheld "" "a Mac whose count started again a look ago was offered before the hold"

pressed_ago 601
look 50 3
holds offered "*offered again*" \
  "a Mac quiet for the hold after its count started again wasn't offered"
look fails 3
holds withheld "*isn't offered: it couldn't read the Mac's room: its share of memory free" \
  "a sysctl that failed left the builder offered"
look 50 3
holds withheld "" "a Mac unread a look ago was offered before the hold"
pressed_ago 601
look fails 3
holds withheld "" "a room still unread was offered, or said again"

pressed_ago 601
look 50 3
holds offered "*offered again*" "a Mac quiet for the hold after an unread room wasn't offered"
look 50 fails
holds withheld "*isn't offered: it couldn't read the Mac's room: its swap-outs" \
  "a vm_stat that failed left the builder offered"

pressed_ago 601
look 50 3
: >"${state}/pressed"
look 50 3
holds withheld "*isn't offered: it had no record of the Mac's room yet" \
  "an emptied record of when the Mac was short skipped the hold"
look 50 3
holds withheld "" "a Mac with no record a look ago was offered before the hold"

pressed_ago 601
look 50 3
pressed_ago -3600
look 50 3
holds withheld "*isn't offered: its record of the Mac's room is from later than now" \
  "a record from an hour ahead kept the builder offered"
look 50 3
holds withheld "" "a Mac whose record was ahead a look ago was offered before the hold"

# A record it can't write, as on a full disk, ends the job with no builder
# offered, said once however many looks fail, and a shortage that look saw
# still runs the hold. Root writes anywhere, so the test needs an account
# that can't.
pressed_ago 601
look 50 3
holds offered "*offered again*" "the builder wasn't offered before a write failed"
chmod 555 "${state}"
if : 2>/dev/null >"${state}/probe"; then
  echo "room-test: a folder can still be written after chmod 555, as root can," \
    "so this tests nothing" >&2
  exit 1
fi
look 34 3
failing="isn't offered: it failed, ending ${ended}"
[[ ${offer} == withheld && ${ended} != 0 && ${said} == *"${failing}" ]] ||
  fail "a record it couldn't write left the builder offered"
look 50 3
# bash says the write it refused; room.sh says nothing of its own again.
[[ ${offer} == withheld && ${ended} != 0 && ${said} != *" room: "* ]] ||
  fail "a second look that couldn't write left the builder offered, or said it again"
chmod 755 "${state}"
look 50 3
holds withheld "" "a shortage a look couldn't write down was forgotten, skipping the hold"

pressed_ago 601
look 50 3
holds offered "*offered again*" "the builder wasn't offered before its line changed"
echo "ssh-ng://linux-builder x86_64-linux ${key} 4 1 big-parallel -" >"${PWD}/line"
look 50 3
holds offered "*line changed*" \
  "a changed line wasn't offered as it is, or was said as offered again"

echo "room-test: the builder was offered only with room, and back only after the hold"
