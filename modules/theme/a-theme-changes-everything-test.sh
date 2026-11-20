#!/usr/bin/env bash
# Cases for a-theme-changes-everything.sh, run before it reads the real home,
# so the check can't come to pass whatever it finds again, as it did while it
# trusted rg's end: rg ends in 2 on a link it can't follow, match or none.
#
#   a-theme-changes-everything-test.sh <script> <colours file>
set -euo pipefail

script=$1
colours=$2
home=$(mktemp -d)
trap 'rm -rf "$home"' EXIT

mkdir "$home/clean" "$home/kept" "$home/silent" "$home/erring"
echo '{"color":"#100F0F"}' > "$home/clean/style.json"
ln -s /nowhere/left "$home/clean/gone"
first=$(head -n 1 "$colours")
echo "{\"color\":\"$first\"}" > "$home/kept/style.json"
ln -s /nowhere/left "$home/kept/gone"
: > "$home/none"
# An rg that dies without a word, as one killed would.
printf '#!%s\nexit 137\n' "$BASH" > "$home/silent/rg"
chmod +x "$home/silent/rg"
# An rg that couldn't read a file, which is an error but not a link it can't
# follow, beside one that is.
printf '#!%s\necho "rg: gone: No such file or directory (os error 2)" >&2\n' "$BASH" \
  > "$home/erring/rg"
printf 'echo "rg: style.json: Permission denied (os error 13)" >&2\nexit 2\n' \
  >> "$home/erring/rg"
chmod +x "$home/erring/rg"

passes() {
  if ! "$BASH" "$script" "$@" > "$home/said" 2>&1; then
    cat "$home/said"
    echo "It failed on a home that should pass: $*"
    exit 1
  fi
}
fails() {
  if "$BASH" "$script" "$@" > "$home/said" 2>&1; then
    echo "It passed on a home that should fail: $*"
    exit 1
  fi
}

passes "$colours" "$home/clean/"
fails "$colours" "$home/kept/"
fails "$colours" "$home/clean/" "$home/kept/style.json"
fails "$home/none" "$home/kept/"
fails "$colours" "$home/clean/" "$home/nowhere"
if PATH="$home/silent:$PATH" "$BASH" "$script" "$colours" "$home/clean/" > "$home/said" 2>&1; then
  echo "It passed though rg died without a word."
  exit 1
fi
if PATH="$home/erring:$PATH" "$BASH" "$script" "$colours" "$home/clean/" > "$home/said" 2>&1; then
  echo "It passed though rg couldn't read a file it was shown."
  exit 1
fi
