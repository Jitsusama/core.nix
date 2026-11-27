# shellcheck shell=bash
# shellcheck disable=SC2310 # take_kernel_files is the test, read in a condition, as it may fail.
# Checks that kernel-source-files.sh takes a kernel's files as the build
# finds them, from a folder and from its tarball alike: a file missing that
# a patch creates is let be, one missing that no patch creates is refused,
# and one the source has that a patch would create is refused, naming the
# patch. A kernel's source is too big to unpack for this, so a tiny one
# stands in for it, with a patch creating one file.
# usage: kernel-source-files-cases.sh <kernel-source-files.sh>
set -euo pipefail
# A failure inside a command substitution stops the cases too, rather than
# passing an empty value along.
shopt -s inherit_errexit

# The library is handed in by its store path; shellcheck reads it from here.
# shellcheck source=SCRIPTDIR/kernel-source-files.sh
source "$1"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/linux/drivers"
echo kept >"$work/linux/drivers/kept.c"
cat >"$work/make.patch" <<'PATCH'
--- /dev/null
+++ b/drivers/made.c
@@ -0,0 +1 @@
+made
PATCH
echo "$work/make.patch" >"$work/patches"
# The same source with the file the patch creates, as a newer kernel's may.
cp -r "$work/linux" "$work/grown"
echo made >"$work/grown/drivers/made.c"
tar cf "$work/linux.tar" -C "$work" linux
tar cf "$work/grown.tar" -C "$work" --transform 's,^grown,linux,' grown

failed=0
# Takes the files named from a source, and holds what it says to what's
# expected: nothing, where it should take them, or words it fails saying.
expect() {
  local name=$1 source=$2 expected=$3 said
  shift 3
  local out
  out=$(mktemp -d -p "$work")
  if said=$(take_kernel_files "$source" "$work/patches" "$out" "$@"); then
    if [ -n "$expected" ]; then
      echo "$name: should fail saying '$expected', and passed."
      failed=1
    elif ! [ -e "$out/drivers/kept.c" ] || [ -e "$out/drivers/made.c" ]; then
      echo "$name: should take only what the source has, and took:"
      find "$out" -type f
      failed=1
    fi
  elif [ -z "$expected" ]; then
    echo "$name: should pass, and failed: $said"
    failed=1
  elif [[ $said != *"$expected"* ]]; then
    echo "$name: should fail saying '$expected', and said: $said"
    failed=1
  fi
}

for form in folder tarball; do
  if [ "$form" = folder ]; then
    source=$work/linux grown=$work/grown
  else
    source=$work/linux.tar grown=$work/grown.tar
  fi
  expect "a file a patch creates, from a $form" "$source" "" \
    drivers/kept.c drivers/made.c
  expect "a file a patch creates that the source has, from a $form" "$grown" \
    "make.patch creates drivers/made.c, which the kernel's source already has." \
    drivers/kept.c drivers/made.c
  expect "a file no patch creates, from a $form" "$source" \
    "drivers/gone.c isn't in the kernel's source, and no patch creates it." \
    drivers/kept.c drivers/gone.c
done
exit "$failed"
