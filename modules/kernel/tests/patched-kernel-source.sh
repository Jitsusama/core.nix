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

mkdir -p "$out"
if [ -d "$source" ]; then
  for file in "$@"; do
    install -D -m 644 "$source/$file" "$out/$file"
  done
else
  wanted=()
  for file in "$@"; do
    wanted+=("*/$file")
  done
  tar xf "$source" -C "$out" --strip-components=1 --wildcards "${wanted[@]}"
fi

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
