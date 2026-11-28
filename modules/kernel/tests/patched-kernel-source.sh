# shellcheck shell=bash
# Gives a few files of a kernel's source as the machine builds them: taken
# from the source, a tarball or a folder, into the output folder, with
# each patch listed applied to just those files. A check then reads what
# the kernel the machine boots does, without unpacking or building all of
# it. The files are taken by kernel-source-files.sh, handed in first.
# usage: patched-kernel-source.sh <kernel-source-files.sh> <source> <patch list> <out> <file>...
set -euo pipefail
# A failure inside a command substitution, such as one of the library's,
# stops the check too, rather than passing an empty value along.
shopt -s inherit_errexit

# The library is handed in by its store path; shellcheck reads it from here.
# shellcheck source=SCRIPTDIR/kernel-source-files.sh
source "$1"
source=$2
patches=$3
out=$4
shift 4

take_kernel_files "$source" "$patches" "$out" "$@"

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
