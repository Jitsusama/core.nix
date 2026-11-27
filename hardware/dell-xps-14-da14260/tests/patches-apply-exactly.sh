# shellcheck shell=bash
# Checks that the patches named apply to a kernel's source exactly, with no
# offset or fuzz, after every patch the build applies before them. A patch
# taken from elsewhere onto a kernel that drifted under it can still apply
# at an offset, or with fuzz, to code it was never tested against, and the
# build says nothing of it.
# usage: patches-apply-exactly.sh <source> <patch list> <exact list> <out>
set -euo pipefail

source=$1
patches=$2
exact=$3
out=$4

if ! [ -s "$exact" ]; then
  echo "No patch was given to hold exactly."
  exit 1
fi

# The files the exact patches change, so only those come out of the source.
listed=$(xargs lsdiff --strip=1 --no-filename <"$exact")
sorted=$(sort -u <<<"$listed")
mapfile -t files <<<"$sorted"
mkdir -p "$out"
if [ -d "$source" ]; then
  for file in "${files[@]}"; do
    install -D -m 644 "$source/$file" "$out/$file"
  done
else
  wanted=()
  for file in "${files[@]}"; do
    wanted+=("*/$file")
  done
  tar xf "$source" -C "$out" --strip-components=1 --wildcards "${wanted[@]}"
fi

only=()
for file in "${files[@]}"; do
  only+=(-i "$file")
done
cd "$out"
failed=0
while IFS= read -r patch; do
  [ -n "$patch" ] || continue
  if grep -q -x -F "$patch" "$exact"; then
    # Its name as it was written, without the store's hash before it.
    name=${patch##*/}
    name=${name#*-}
    # Applied whole, as the build does, but held to its hunks' own lines.
    said=$(patch -p1 --forward --batch --fuzz=0 <"$patch" 2>&1) || {
      echo "$name no longer applies with no fuzz:"
      echo "$said"
      failed=1
      continue
    }
    # Only patch's report of a hunk is read, since a file's own name can hold
    # either word.
    if grep -Eq '^Hunk #[0-9]+ succeeded at [0-9]+ (\(offset|with fuzz)' <<<"$said"; then
      echo "$name applies only at an offset or with fuzz:"
      echo "$said"
      failed=1
    fi
  else
    filterdiff -p1 "${only[@]}" "$patch" | patch -p1 --forward --batch --silent
  fi
done <"$patches"
exit "$failed"
