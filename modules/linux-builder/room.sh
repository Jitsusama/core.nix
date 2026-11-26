# shellcheck shell=bash
# Offers the Linux builder's line to the Mac's Nix only while the Mac has
# room, run by launchd every 15 seconds. The VM has no balloon, so builds
# started in it while the Mac is short of memory push the Mac's own
# programs into swap: on 25 November a Mac at 26% free, swapping, froze its
# person's window while its VM ran builds sent to it. So under the line,
# with less than the floor's share of memory free (macOS's own reckoning,
# kern.memorystatus_level) or any page swapped out since the last look, the
# file Nix reads its builders from holds no builder, and no new build is
# sent to the VM; builds already running go on. The line comes back only
# once the Mac has stayed clear of it for the hold, so a Mac recovering isn't
# loaded again the moment it breathes. Free swap isn't read: macOS drops
# swap files as they empty, so it reads short just as the Mac recovers. A
# room it can't read, or a record of its own it can't trust, counts as
# short, so it never offers blind.
#
# Nix's build hook reads the file as it starts (build-remote.cc's
# getMachines), and a hook that took a build ends with it, so the next
# build starts a hook that reads it again, with no daemon restart. A hook
# that declined or waited for a free slot is kept for that Nix's next
# build with the list it read, so a Nix already waiting may still send the
# VM one build after it's withheld. One that started while it was withheld
# found no builder, and its Nix builds no Linux there until it ends.
# usage: room.sh <sysctl> <vm_stat> <line> <offered> <state folder> <floor %> <hold s>
set -euo pipefail

sysctl=$1
vm_stat=$2
line=$3
offered=$4
state=$5
floor=$6
hold=$7

# A step that fails past here, as a write on a full disk, ends the job
# with what Nix reads emptied in place but for a comment saying so, said
# once, rather than with the last offer standing: emptying a file in place
# takes no room.
failed() {
  local status=$? when
  [ "${status}" -ne 0 ] || return 0
  if grep -qv '^#' "${offered}" 2>/dev/null; then
    echo "# withheld by linux-builder-room: it failed, ending ${status}" >"${offered}" ||
      true
    when=$(date -u +%Y-%m-%dT%H:%M:%SZ) || when=""
    echo "${when} room: the builder isn't offered: it failed, ending ${status}" >&2 || true
  fi
}
trap failed EXIT

now=${EPOCHSECONDS}
# Either read failing is a room not read, said below, rather than an end
# that leaves the last offer standing.
free=$("${sysctl}" -n kern.memorystatus_level 2>/dev/null) || free=""
swapouts=$("${vm_stat}" 2>/dev/null | sed -n 's/^Swapouts:[[:space:]]*\([0-9]*\)\.$/\1/p') ||
  swapouts=""
number='^[0-9]+$'

# When the Mac was last short of room, and the swap-outs it had then. A
# first look, a record that isn't two numbers or one from later than now,
# as after the clock was set back, counts as short, so the hold runs
# before the builder is offered.
pressed=""
seen=""
if [ -r "${state}/pressed" ] && [ -r "${state}/swapouts" ]; then
  pressed=$(<"${state}/pressed")
  seen=$(<"${state}/swapouts")
fi
why=""
# What couldn't be read is named, never what it printed instead, which
# would go into the file Nix reads.
unread=""
[[ ${free} =~ ${number} ]] || unread="its share of memory free"
[[ ${swapouts} =~ ${number} ]] || unread="${unread:+${unread} and }its swap-outs"
if [ -n "${unread}" ]; then
  why="it couldn't read the Mac's room: ${unread}"
  swapouts=${seen}
elif ! [[ ${pressed} =~ ${number} && ${seen} =~ ${number} ]]; then
  why="it had no record of the Mac's room yet"
elif [ "${pressed}" -gt "${now}" ]; then
  why="its record of the Mac's room is from later than now"
elif [ "${free}" -lt "${floor}" ]; then
  why="${free}% free, under ${floor}%"
elif [ "${swapouts}" -lt "${seen}" ]; then
  # The count starts again with the Mac, so it may have swapped since.
  why="the swap-out count started again, as the Mac does"
elif [ "${swapouts}" -gt "${seen}" ]; then
  why="$((swapouts - seen)) pages swapped out"
fi
if [ -n "${why}" ]; then
  pressed=${now}
fi

# Written aside and moved into place, so neither Nix nor the next look
# reads half a file.
printf '%s\n' "${pressed}" >"${state}/pressed.new"
mv "${state}/pressed.new" "${state}/pressed"
printf '%s\n' "${swapouts}" >"${state}/swapouts.new"
mv "${state}/swapouts.new" "${state}/swapouts"

# Short of room now, or within the hold, holds no builder: a comment Nix
# skips, saying why, for whoever reads the file.
withheld=${why:-the Mac was short of room under ${hold} s ago}
if [ -z "${why}" ] && [ $((now - pressed)) -ge "${hold}" ]; then
  cp "${line}" "${offered}.new"
else
  echo "# withheld by linux-builder-room: ${withheld}" >"${offered}.new"
fi

# Said only when what Nix reads changes, whether it holds the builder or
# not, so a Mac that stays short says so once. A file not there yet, as
# at the first look, counts as a change either way.
was=$(grep -v '^#' "${offered}" 2>/dev/null || true)
is=$(grep -v '^#' "${offered}.new" || true)
if [ "${is}" != "${was}" ] || [ ! -e "${offered}" ]; then
  when=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  if [ -z "${is}" ]; then
    echo "${when} room: the builder isn't offered: ${withheld}" >&2
  elif [ -z "${was}" ]; then
    echo "${when} room: the builder is offered again, the Mac at ${floor}% free or more" \
      "with nothing swapped out for ${hold} s" >&2
  else
    echo "${when} room: the builder's line changed, and is offered as it is now" >&2
  fi
fi
mv "${offered}.new" "${offered}"
