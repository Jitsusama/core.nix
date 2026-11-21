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

mkdir "$home/clean" "$home/kept" "$home/linked" "$home/silent" "$home/erring"
echo '{"color":"#100F0F"}' > "$home/clean/style.json"
ln -s /nowhere/left "$home/clean/gone"
first=$(head -n 1 "$colours")
echo "{\"color\":\"$first\"}" > "$home/kept/style.json"
ln -s /nowhere/left "$home/kept/gone"
# A home's files are links into the store, many under dot-folders, so a kept
# colour is reached only through a link in one.
mkdir "$home/linked/.config" "$home/store"
mv "$home/kept/style.json" "$home/store/style.json"
ln -s "$home/store/style.json" "$home/kept/style.json"
ln -s "$home/store/style.json" "$home/linked/.config/style.json"
ln -s /nowhere/left "$home/linked/gone"
# A file may write a colour in either case, so one written in the other
# case from the theme's is kept all the same: a theme's colour in capitals
# written small, or one written small in capitals, each where the theme has
# one. rg's --smart-case reads the whole list case-sensitively once any
# colour in it has a capital, so the first catches it; with none, it's
# --ignore-case by another name.
written=()
cased() {
  local lettered flipped
  lettered=$(grep -m 1 "$2" "$colours") || return 0
  flipped=$(printf '%s' "$lettered" | tr 'A-Fa-f' 'a-fA-F')
  if grep -q -x -F "$flipped" "$colours"; then
    echo "$flipped is a colour of its own in $colours, so it can't show case is let go."
    exit 1
  fi
  mkdir "$home/$1"
  echo "{\"color\":\"$flipped\"}" > "$home/$1/style.json"
  written+=("$1")
}
cased small '[A-F]'
cased capitals '[a-f]'
if [ "${#written[@]}" = 0 ]; then
  echo "No colour in $colours holds a letter to write in the other case."
  exit 1
fi
: > "$home/none"
# An rg that ends in error without a word.
printf '#!%s\nexit 2\n' "$BASH" > "$home/silent/rg"
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
fails "$colours" "$home/linked/"
# Each home in the other case fails for the colour it keeps, not for some
# other reason.
for case in "${written[@]}"; do
  fails "$colours" "$home/$case/"
  if ! grep -q "kept Osaka Jade's colours" "$home/said"; then
    cat "$home/said"
    echo "The home written in $case failed, but not for the colour it keeps; it said the above."
    exit 1
  fi
done
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
