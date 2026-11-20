# shellcheck shell=bash
# Runs the harness's settings program (modules/pi/harness-settings.nix) over
# a config the harness wrote itself: the declared setting goes in beside its
# section's other keys, and every other section and key stays.
# usage: pi-harness-takes-its-settings.sh <the program>
set -euo pipefail

program=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
file=$work/config.json

echo '{"version":1,"sections":{"review":{"a":1},"quest-workflow":{"kept":true}}}' > "$file"
"$program" "$file"
jq -e '.version == 1 and .sections.review == {"a": 1}
  and .sections."quest-workflow" == {"kept": true, "budgetWhenCapped": "a-budget"}' \
  "$file" > /dev/null \
  || {
    written=$(< "$file")
    echo "the harness's config lost a key or missed the declared one: $written"
    exit 1
  }
