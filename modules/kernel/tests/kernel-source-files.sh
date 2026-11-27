# shellcheck shell=bash
# Takes a few files of a kernel's source, from its tarball or a folder, into
# an output folder, as the build would find them before its patches: what
# the source has, and nothing for a file a patch creates. A check sources it
# to read what the kernel the machine boots does without unpacking it all.
# usage, once sourced: take_kernel_files <source> <patch list> <out> <file>...

# A failure inside a command substitution, such as lsdiff's, stops the
# script sourcing this too, rather than passing an empty list along.
shopt -s inherit_errexit

# A patch's name as it was written, without the store's hash before it.
patch_name() {
  local name=${1##*/}
  if [[ $name =~ ^[0-9a-z]{32}-(.+)$ ]]; then
    name=${BASH_REMATCH[1]}
  fi
  echo "$name"
}

# A file some patch creates may be missing from the source: that patch
# makes it. One the source has too is refused, naming the patch, since the
# build refuses a patch creating a file that's already there; and a file
# missing that no patch creates is refused, as a check would read nothing.
take_kernel_files() {
  local source=$1 patches=$2 out=$3
  shift 3
  local -A creates=()
  local patch statuses created file name said=""
  while IFS= read -r patch; do
    [ -n "$patch" ] || continue
    statuses=$(lsdiff --status --strip=1 --no-filename "$patch")
    created=$(sed -n 's/^+ //p' <<<"$statuses")
    while IFS= read -r file; do
      if [ -n "$file" ]; then
        creates[$file]=$patch
      fi
    done <<<"$created"
  done <"$patches"

  mkdir -p "$out"
  if [ -d "$source" ]; then
    for file in "$@"; do
      if [ -e "$source/$file" ]; then
        install -D -m 644 "$source/$file" "$out/$file"
      fi
    done
  else
    local wanted=()
    for file in "$@"; do
      wanted+=("*/$file")
    done
    said=$(tar xf "$source" -C "$out" --strip-components=1 --wildcards "${wanted[@]}" 2>&1) || true
  fi
  for file in "$@"; do
    if [ -e "$out/$file" ] && [ -n "${creates[$file]:-}" ]; then
      name=$(patch_name "${creates[$file]}")
      echo "$name creates $file, which the kernel's source already has."
      return 1
    fi
    if ! [ -e "$out/$file" ] && [ -z "${creates[$file]:-}" ]; then
      echo "$file isn't in the kernel's source, and no patch creates it."
      echo "$said"
      return 1
    fi
  done
}
