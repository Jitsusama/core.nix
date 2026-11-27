# shellcheck shell=bash
# Checks that the patches named apply to a kernel's source exactly, with no
# offset or fuzz, after every patch the build applies before them. A patch
# taken from elsewhere onto a kernel that drifted under it can still apply
# at an offset, or with fuzz, to code it was never tested against, and the
# build says nothing of it. The files are taken by core's kernel's
# kernel-source-files.sh, handed in first.
# usage: patches-apply-exactly.sh <kernel-source-files.sh> <source> <patch list> <exact list> <out>
set -euo pipefail
# A failure inside a command substitution, such as one of the library's,
# stops the check too, rather than passing an empty value along.
shopt -s inherit_errexit

# The library is handed in by its store path; shellcheck reads it from here.
# shellcheck source=SCRIPTDIR/../../../modules/kernel/tests/kernel-source-files.sh
source "$1"
source=$2
patches=$3
exact=$4
out=$5

if ! [ -s "$exact" ]; then
  echo "No patch was given to hold exactly."
  exit 1
fi

# The files the exact patches change, so only those come out of the source.
listed=$(xargs lsdiff --strip=1 --no-filename <"$exact")
sorted=$(sort -u <<<"$listed")
mapfile -t files <<<"$sorted"
take_kernel_files "$source" "$patches" "$out" "${files[@]}"

only=()
for file in "${files[@]}"; do
  only+=(-i "$file")
done
cd "$out"
failed=0
while IFS= read -r patch; do
  [ -n "$patch" ] || continue
  if grep -q -x -F "$patch" "$exact"; then
    name=$(patch_name "$patch")
    # Applied whole, as the build does, but held to its hunks' own lines.
    said=$(patch -p1 --forward --batch --fuzz=0 <"$patch" 2>&1) || {
      echo "$name no longer applies with no fuzz, after the patches before it:"
      echo "$said"
      failed=1
      continue
    }
    # Only patch's report of a hunk is read, since a file's own name can hold
    # either word.
    if grep -Eq '^Hunk #[0-9]+ succeeded at [0-9]+ (\(offset|with fuzz)' <<<"$said"; then
      echo "$name applies only at an offset or with fuzz, after the patches before it:"
      echo "$said"
      failed=1
    fi
  else
    filterdiff -p1 "${only[@]}" "$patch" | patch -p1 --forward --batch --silent
  fi
done <"$patches"
exit "$failed"
