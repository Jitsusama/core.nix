# shellcheck shell=bash
# Checks the job that keeps a Mac's local name, with a stand-in scutil that
# keeps LocalHostName in a file, fails as the real one does when it isn't
# set or configd is busy, and records each set: a Mac macOS renamed a-mac-2
# gets a-mac back and says so once; one already named a-mac is left
# unwritten; one renamed again within five minutes of being set back is left
# until they've passed; an unset name is set; and a name that can't be read
# is said and left alone.
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
    if [[ ! -e "$work/name" ]]; then echo "LocalHostName: not set"; exit 1; fi
    cat "$work/name" ;;
  "--set LocalHostName") printf '%s\n' "\$3" > "$work/name"; echo set >> "$work/sets" ;;
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
}
# Says the last set back was longer ago than five minutes.
aged() {
  touch -d '6 minutes ago' "$work/stamp"
}
# Fails saying $2 unless what was said is the one line the job writes for a
# rename from $1.
said_once() {
  local lines
  lines=$(wc -l < "$work/said")
  if [[ ${lines} -ne 1 ]] ||
    ! grep -Eq "^[0-9T:-]+Z the local name was $1, so it is a-mac again$" "$work/said"; then
    fail "$2: $(<"$work/said")"
  fi
}

echo a-mac-2 > "$work/name"
keeps
[[ $(<"$work/name") == a-mac ]] || fail "a renamed Mac's name stayed $(<"$work/name")"
sets=$(wc -l < "$work/sets")
[[ ${sets} -eq 1 ]] || fail "setting a renamed Mac's name back set it ${sets} times"
said_once a-mac-2 "setting the name back didn't say so once"

keeps
[[ ! -s "$work/sets" && ! -s "$work/said" ]] ||
  fail "a name already right was written again, or the job said something: $(<"$work/said")"

echo a-mac-2 > "$work/name"
keeps
[[ $(<"$work/name") == a-mac-2 && ! -s "$work/sets" && ! -s "$work/said" ]] ||
  fail "a name renamed again within five minutes of being set back was set back again"
aged
keeps
[[ $(<"$work/name") == a-mac ]] ||
  fail "a name renamed again five minutes after being set back wasn't set back"
said_once a-mac-2 "setting the name back again didn't say so once"

rm "$work/name"
aged
keeps
[[ -e "$work/name" && $(<"$work/name") == a-mac ]] || fail "an unset name wasn't set"
said_once unset "setting an unset name didn't say so once"

echo a-mac-2 > "$work/name"
touch "$work/busy"
aged
keeps
[[ ${ended} -ne 0 && $(<"$work/name") == a-mac-2 && ! -s "$work/sets" ]] ||
  fail "a name that couldn't be read was written over"
grep -q "couldn't read the local name: SCPreferences: busy" "$work/said" ||
  fail "a name that couldn't be read wasn't said: $(<"$work/said")"
