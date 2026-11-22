# shellcheck shell=bash
# Checks the job that keeps a Mac's local name, with a stand-in scutil that
# keeps LocalHostName in a file, fails as the real one does when it isn't
# set or configd is busy, can be made to warn beside a name it reads or to
# refuse a set, and records each set it's asked for: a Mac macOS renamed
# a-mac-2 gets a-mac back and says so once; one already named a-mac is left
# unwritten, its last set-back's time kept, however old, and a warning
# beside it isn't read as part of it; one renamed again four minutes after being set back is left,
# its time kept, until five have passed; a time still to come counts as
# old; an unset name is set; a name that can't be read is said once in five
# minutes, by a time of its own, and left alone; one that can be read again
# a moment on isn't said; and a set that's refused is said and tried once
# in five minutes.
# usage: local-name-comes-back.sh <keep.sh> <bash>
set -euo pipefail

keep=$1
bash=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
cat > "$work/bin/scutil" <<EOF
#!${bash}
case "\$1 \$2" in
  "--get LocalHostName")
    if [[ -e "$work/busy" ]]; then echo "SCPreferences: busy"; exit 1; fi
    if [[ -e "$work/busy-once" ]]; then rm "$work/busy-once"; echo "SCPreferences: busy"; exit 1; fi
    if [[ ! -e "$work/name" ]]; then echo "LocalHostName: not set"; exit 1; fi
    if [[ -e "$work/noisy" ]]; then echo "SCPreferences: a warning" >&2; fi
    cat "$work/name" ;;
  "--set LocalHostName")
    echo set >> "$work/sets"
    if [[ -e "$work/refuse" ]]; then echo "SCPreferences: refused" >&2; exit 1; fi
    printf '%s\n' "\$3" > "$work/name" ;;
  *) exit 64 ;;
esac
EOF
chmod +x "$work/bin/scutil"
export PATH="$work/bin:$PATH"

fail() {
  echo "local-name-comes-back: $1" >&2
  exit 1
}
# Runs the job, its exit in ended, what it said in said and its sets in sets.
keeps() {
  : > "$work/sets"
  ended=0
  "$bash" "$keep" a-mac "$work/stamp" 2> "$work/said" || ended=$?
  sets=$(wc -l < "$work/sets")
}
# Says the last set back was $1 seconds ago; a negative $1 is still to come.
set_back_ago() {
  local now
  now=$(date +%s)
  echo "$((now - $1))" > "$work/stamp"
}
# Fails saying $2 unless the job set the name once and said the one line it
# writes for a rename from $1.
set_back_once() {
  local lines
  [[ ${sets} -eq 1 ]] || fail "$2: it set the name ${sets} times"
  lines=$(wc -l < "$work/said")
  [[ ${lines} -eq 1 ]] || fail "$2: it said ${lines} lines: $(<"$work/said")"
  grep -Eq "^[0-9T:-]+Z the local name was $1, so it is a-mac again$" "$work/said" ||
    fail "$2: it said: $(<"$work/said")"
}
# Fails saying $1 unless the job neither set the name nor said anything.
left_quietly() {
  [[ ${sets} -eq 0 ]] || fail "$1: it set the name ${sets} times"
  [[ ! -s "$work/said" ]] || fail "$1: it said: $(<"$work/said")"
}

echo a-mac-2 > "$work/name"
keeps
[[ $(<"$work/name") == a-mac ]] || fail "a renamed Mac's name stayed $(<"$work/name")"
set_back_once a-mac-2 "setting a renamed Mac's name back"
[[ ${ended} -eq 0 ]] || fail "setting a renamed Mac's name back ended ${ended}"

set_back_ago 360
kept=$(<"$work/stamp")
keeps
left_quietly "a name already right, last set back six minutes ago"
[[ $(<"$work/stamp") == "${kept}" ]] ||
  fail "a name already right had its last set-back's time moved, holding a later rename off"

touch "$work/noisy"
keeps
left_quietly "a name already right, read beside a warning"
rm "$work/noisy"

echo a-mac-2 > "$work/name"
set_back_ago 240
kept=$(<"$work/stamp")
keeps
[[ $(<"$work/name") == a-mac-2 ]] ||
  fail "a name renamed again four minutes after being set back was set back again"
left_quietly "a name renamed again four minutes after being set back"
[[ $(<"$work/stamp") == "${kept}" ]] ||
  fail "holding a name off moved its last set-back's time, so it would never come back"
set_back_ago 300
keeps
[[ $(<"$work/name") == a-mac ]] ||
  fail "a name renamed again five minutes after being set back wasn't set back"
set_back_once a-mac-2 "setting the name back again five minutes on"

echo a-mac-2 > "$work/name"
set_back_ago -3600
keeps
[[ $(<"$work/name") == a-mac ]] ||
  fail "a last set-back dated an hour to come, as after the clock went back, held the name off"
set_back_once a-mac-2 "setting the name back past a time to come"

rm "$work/name"
set_back_ago 360
keeps
[[ -e "$work/name" ]] || fail "an unset name wasn't set"
[[ $(<"$work/name") == a-mac ]] || fail "an unset name was set to $(<"$work/name")"
set_back_once unset "setting an unset name"

echo a-mac-2 > "$work/name"
touch "$work/busy"
set_back_ago 360
kept=$(<"$work/stamp")
keeps
[[ ${ended} -ne 0 ]] || fail "a name that couldn't be read ended as if it had been kept"
[[ $(<"$work/name") == a-mac-2 ]] || fail "a name that couldn't be read was written over"
[[ ${sets} -eq 0 ]] || fail "a name that couldn't be read was set ${sets} times"
grep -q "couldn't read the local name: SCPreferences: busy" "$work/said" ||
  fail "a name that couldn't be read wasn't said: $(<"$work/said")"
keeps
[[ ${ended} -ne 0 ]] || fail "a name still unreadable ended as if it had been kept"
[[ ! -s "$work/said" ]] ||
  fail "a name still unreadable a moment later was said again: $(<"$work/said")"
[[ $(<"$work/stamp") == "${kept}" ]] ||
  fail "a name that couldn't be read moved the last set-back's time, holding the name off"
said_at=$(date +%s)
echo "$((said_at - 300))" > "$work/stamp.unread"
keeps
grep -q "couldn't read the local name: SCPreferences: busy" "$work/said" ||
  fail "a name still unreadable five minutes on wasn't said again: $(<"$work/said")"
rm "$work/busy" "$work/stamp.unread"

touch "$work/busy-once"
keeps
[[ ${sets} -eq 0 ]] || fail "a name readable again a moment on was set ${sets} times"
[[ ! -s "$work/said" ]] ||
  fail "a name readable again a moment on was said as unreadable: $(<"$work/said")"

touch "$work/refuse"
set_back_ago 360
keeps
[[ ${ended} -ne 0 ]] || fail "a refused set ended as if the name were back"
[[ ${sets} -eq 1 ]] || fail "a refused set was tried ${sets} times"
grep -q "couldn't set the local name back to a-mac from a-mac-2" "$work/said" ||
  fail "a refused set wasn't said: $(<"$work/said")"
keeps
[[ ${sets} -eq 0 ]] || fail "a refused set was tried again within five minutes"
[[ ! -s "$work/said" ]] || fail "a refused set was said again within five minutes: $(<"$work/said")"
