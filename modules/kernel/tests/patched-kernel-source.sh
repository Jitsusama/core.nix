# shellcheck shell=bash
# Gives a few files of a kernel's source as the machine builds them: taken
# from the source, a tarball or a folder, into the output folder, with
# each patch listed applied to just those files. A check then reads what
# the kernel the machine boots does, without unpacking or building all of
# it.
# usage: patched-kernel-source.sh <source> <patch list> <out> <file>...
set -euo pipefail

source=$1
patches=$2
out=$3
shift 3

# A file some patch creates may be missing from the source: that patch
# makes it. One the source has too is taken all the same, so the patch
# fails to create it here as it would in the build.
statuses=$(xargs -r lsdiff --status --strip=1 --no-filename <"$patches")
created=$(sed -n 's/^+ //p' <<<"$statuses")

mkdir -p "$out"
if [ -d "$source" ]; then
  for file in "$@"; do
    if [ -e "$source/$file" ]; then
      install -D -m 644 "$source/$file" "$out/$file"
    fi
  done
  said=""
else
  wanted=()
  for file in "$@"; do
    wanted+=("*/$file")
  done
  said=$(tar xf "$source" -C "$out" --strip-components=1 --wildcards "${wanted[@]}" 2>&1) || true
fi
for file in "$@"; do
  if ! [ -e "$out/$file" ] && ! grep -q -x -F "$file" <<<"$created"; then
    echo "$file isn't in the kernel's source, and no patch creates it."
    echo "$said"
    exit 1
  fi
done

# Each patch, cut to the files given, as the build applies it.
only=()
for file in "$@"; do
  only+=(-i "$file")
done
cd "$out"
while IFS= read -r patch; do
  [ -n "$patch" ] || continue
  filterdiff -p1 "${only[@]}" "$patch" | patch -p1 --forward --batch --silent
done < "$patches"
