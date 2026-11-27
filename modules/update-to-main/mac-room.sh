# shellcheck shell=bash
# Whether a Mac is short of the memory to build its own update in, asked
# before the follower's ask fetches anything (darwin.nix). Its load can't
# say: on 26 November fseventsd and a music app's renderer held it past
# three a processor for hours with memory to spare, refusing every ask. So
# it's judged by the share of memory macOS reckons free
# (kern.memorystatus_level) and the pages it swapped out, read twice at
# least a minute apart: it has room only when both readings are at the
# floor or over it and nothing was swapped out between them, so one
# passing blip neither lets a build in nor holds one off. Each look keeps
# its reading in the record for the next to pair with; the follower asks
# again a minute after a refusal, so a Mac with room is let through on the
# second ask.
#
# A question, answered as `test` does, which update-to-main reads: 0 while
# the Mac is short, 1 while it has room, and 69 when it can't tell, which
# update-to-main refuses as well. Why goes to standard error either way.
#
# usage: mac-room.sh <sysctl> <vm_stat> <record> <floor %>
set -euo pipefail

# A step that fails anywhere, as a record that can't be written or an
# argument left out, ends as can't tell rather than with its own status,
# since 1 would read as room. The verdict is set only once its words are
# out, so a stream that can't take them can't tell either.
verdict=""
trap '[ -n "${verdict}" ] || exit 69' EXIT
answer() {
  echo "mac-room: $2" >&2
  verdict=$1
  exit "$1"
}

sysctl=$1
vm_stat=$2
record=$3
floor=$4

# Readings closer than this aren't a minute apart; older than the most,
# the first is too stale to say the Mac stayed clear since.
apart=55
most=600

now=${EPOCHSECONDS}
free=$("${sysctl}" -n kern.memorystatus_level 2>/dev/null) || free=""
# Read by bash itself, since the daemon's PATH may hold any sed.
stats=$("${vm_stat}" 2>/dev/null) || stats=""
swapouts=""
swapped='^Swapouts: *([0-9]+)\.$'
while IFS= read -r line; do
  [[ ! ${line} =~ ${swapped} ]] || swapouts=${BASH_REMATCH[1]}
done <<<"${stats}"
number='^[0-9]+$'
# A floor that isn't a number would fail its test inside an if, which
# falls through to room.
[[ ${floor} =~ ${number} ]] || answer 69 "its floor, ${floor}, isn't a whole number"
# What couldn't be read is named, never what it printed instead.
unread=""
[[ ${free} =~ ${number} ]] || unread="its share of memory free"
[[ ${swapouts} =~ ${number} ]] || unread="${unread:+${unread} and }its swap-outs"
[ -z "${unread}" ] || answer 69 "it couldn't read the Mac's room: ${unread}"

read_at="" was="" seen=""
if [ -r "${record}" ]; then
  read -r read_at was seen <"${record}" || true
fi

# Kept aside and moved into place, so the next look never reads half.
keep() {
  mkdir -p "${record%/*}"
  printf '%s %s %s\n' "${now}" "${free}" "${swapouts}" >"${record}.new"
  mv "${record}.new" "${record}"
}

if ! [[ ${read_at} =~ ${number} && ${was} =~ ${number} && ${seen} =~ ${number} ]]; then
  keep
  answer 0 "it has one reading of the Mac's room so far, ${free}% free"
fi
if [ "${read_at}" -gt "${now}" ]; then
  keep
  answer 0 "its last reading is from later than now, so it starts again"
fi
age=$((now - read_at))
if [ "${age}" -lt "${apart}" ]; then
  # The earlier reading is kept, so the next look a minute on pairs with it.
  answer 0 "its last reading was ${age} s ago, under ${apart} s"
fi
keep
if [ "${age}" -gt "${most}" ]; then
  answer 0 "its last reading was ${age} s ago, past ${most} s, so it starts again"
fi
if [ "${swapouts}" -lt "${seen}" ]; then
  answer 0 "the swap-out count started again, as the Mac does, so it may have swapped"
fi
if [ "${swapouts}" -gt "${seen}" ]; then
  answer 0 "$((swapouts - seen)) pages swapped out in ${age} s"
fi
if [ "${was}" -lt "${floor}" ] || [ "${free}" -lt "${floor}" ]; then
  answer 0 "${was}% then ${free}% free over ${age} s, under ${floor}%"
fi
answer 1 "${was}% then ${free}% free over ${age} s with nothing swapped out"
