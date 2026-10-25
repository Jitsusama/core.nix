# shellcheck shell=bash
# Runs pi's settings program (modules/pi/settings.nix) over a settings file
# as pi leaves it. Core's settings go in and every key pi wrote stays; a
# file already holding them isn't written again; one that isn't JSON is
# refused and left as it was; a missing one is made; and a machine's own
# settings, given to the second program, go in beside core's.
# usage: pi-keeps-its-own-settings.sh <core's program> <a machine's program>
set -euo pipefail

program=$1
machines=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/agent"
file=$work/agent/settings.json

fail() {
  echo "$1"
  exit 1
}

# Holds the file to a jq test, saying what's wrong when it fails.
holds() {
  jq -e "$1" "$file" > /dev/null || fail "$2: $(cat "$file")"
}

# As pi writes it, with a reserve of its own that core's replaces.
echo '{"theme":"jitsusama","lastChangelogVersion":"1.3.0",'\
'"compaction":{"enabled":true,"reserveTokens":16384}}' > "$file"
"$program" "$file"
holds '.theme == "jitsusama" and .lastChangelogVersion == "1.3.0"' "a key pi wrote went"
holds '.compaction == {"enabled": true, "reserveTokens": 64000}' "core's compaction didn't go in"

touch -d 2000-01-01 "$file"
"$program" "$file"
[ "$(stat -c %Y "$file")" = "$(date -d 2000-01-01 +%s)" ] ||
  fail "a file already holding core's settings was written again"

echo '{"theme":' > "$file"
if "$program" "$file"; then
  fail "a file that isn't JSON was taken"
fi
[ "$(cat "$file")" = '{"theme":' ] || fail "a file that isn't JSON was changed: $(cat "$file")"

rm "$file"
"$program" "$file"
holds '.compaction.reserveTokens == 64000' "a missing file wasn't made with core's settings"

echo '{"theme":"jitsusama","defaultModel":"picked-in-pi"}' > "$file"
"$machines" "$file"
holds '.theme == "jitsusama" and .defaultModel == "a-model"' "a machine's setting didn't go in"
holds '.compaction.reserveTokens == 64000' "core's settings didn't go in beside a machine's"
