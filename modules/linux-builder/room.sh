# shellcheck shell=bash
# Offers the Linux builder's line to the Mac's Nix only while the Mac has
# room, run by launchd every 15 seconds. The VM has no balloon, so builds
# started in it while the Mac is short of memory push the Mac's own
# programs into swap: on 25 November a Mac at 26% free, swapping, froze its
# person's window while its VM ran builds sent to it. So under the line,
# with less than the floor's share of memory free (macOS's own reckoning,
# kern.memorystatus_level) or any page swapped out since the last look, the
# file Nix reads its builders from is emptied, and no build starts in the
# VM; builds already running go on. The line comes back only once the Mac
# has stayed above it for the hold, so a Mac recovering isn't loaded again
# the moment it breathes. Free swap isn't read: macOS drops swap files as
# they empty, so it reads short just as the Mac recovers.
#
# Nix reads the file again for each build it may send (build-remote.cc's
# getMachines, once per hook), so an emptied file stops new builds without
# restarting the daemon. A build that needs the builder then fails at once,
# saying no machine can build it, rather than waiting.
# usage: room.sh <sysctl> <vm_stat> <line> <offered> <state folder> <floor %> <hold s>
set -euo pipefail

sysctl=$1
vm_stat=$2
line=$3
offered=$4
state=$5
floor=$6
hold=$7

now=${EPOCHSECONDS}
free=$("${sysctl}" -n kern.memorystatus_level)
swapouts=$("${vm_stat}" | sed -n 's/^Swapouts:[[:space:]]*\([0-9]*\)\.$/\1/p')
if ! [[ ${free} =~ ^[0-9]+$ && ${swapouts} =~ ^[0-9]+$ ]]; then
  echo "room: couldn't read the Mac's room (free ${free:-nothing}, swap-outs ${swapouts:-nothing}), so the builder isn't offered" >&2
  : >"${offered}.new"
  mv "${offered}.new" "${offered}"
  exit 1
fi

# When the Mac was last short of room, and the swap-outs it had then. A
# first look counts as short, so the hold runs before the first offer.
pressed=${now}
seen=${swapouts}
if [ -r "${state}/pressed" ] && [ -r "${state}/swapouts" ]; then
  pressed=$(<"${state}/pressed")
  seen=$(<"${state}/swapouts")
fi
why=""
if [ "${free}" -lt "${floor}" ]; then
  why="${free}% free, under ${floor}%"
elif [ "${swapouts}" != "${seen}" ]; then
  why="$((swapouts - seen)) pages swapped out"
fi
if [ -n "${why}" ]; then
  pressed=${now}
fi
echo "${pressed}" >"${state}/pressed"
echo "${swapouts}" >"${state}/swapouts"

# Written aside and moved into place, so Nix never reads half a line.
if [ $((now - pressed)) -ge "${hold}" ]; then
  cp "${line}" "${offered}.new"
else
  : >"${offered}.new"
fi
if ! cmp -s "${offered}.new" "${offered}"; then
  when=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  if [ -s "${offered}.new" ]; then
    echo "${when} room: the builder is offered again, the Mac above ${floor}% free with nothing swapped out for ${hold} s" >&2
  else
    echo "${when} room: the builder isn't offered, ${why:-the Mac short of room under ${hold} s ago}" >&2
  fi
fi
mv "${offered}.new" "${offered}"
