# shellcheck shell=bash
# shellcheck disable=SC2310 # take_kernel_files is the test, read in a condition, as it may fail.
# Checks that kernel-source-files.sh takes a kernel's files as the build
# finds them, from a folder and from its tarball alike: a file missing that
# a patch creates is let be, one missing that no patch creates is refused,
# one the source has that a patch would create is refused, naming the patch
# without the store's hash, and one a patch before it deletes is let be. A
# kernel's source is too big to unpack for this, so a tiny one stands in
# for it, with a patch creating one file and another deleting it.
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
cat >"$work/delete.patch" <<'PATCH'
--- a/drivers/made.c
+++ /dev/null
@@ -1 +0,0 @@
-made
PATCH
echo "$work/make.patch" >"$work/patches"
# The build hands each patch in by its store path, whose hash goes before
# its name.
store=$work/nix/store
mkdir -p "$store"
cp "$work/make.patch" "$store/0123456789abcdfghijklmnpqrsvwxyz-make.patch"
echo "$store/0123456789abcdfghijklmnpqrsvwxyz-make.patch" >"$work/stored"
printf '%s\n' "$work/delete.patch" "$work/make.patch" >"$work/remade"
printf '%s\n' "$work/make.patch" "$work/delete.patch" >"$work/unmade"
# The build stops at the first patch that creates a file already there.
cp "$work/make.patch" "$work/again.patch"
printf '%s\n' "$work/make.patch" "$work/again.patch" >"$work/twice"
# The same source with the file the patch creates, as a newer kernel's may.
cp -r "$work/linux" "$work/grown"
echo made >"$work/grown/drivers/made.c"
tar cf "$work/linux.tar" -C "$work" linux
tar cf "$work/grown.tar" -C "$work" --transform 's,^grown,linux,' grown

failed=0
# Takes the files named from a source with the patches listed, and holds
# what it says to what's expected: the files it should take, as "takes" and
# their names, where it should pass, or the words its failure starts with.
expect() {
  local name=$1 source=$2 patches=$3 expected=$4 said took
  shift 4
  local out
  out=$(mktemp -d -p "$work")
  if said=$(take_kernel_files "$source" "$patches" "$out" "$@"); then
    took=$(cd "$out" && find . -type f | sed 's,^\./,,' | sort | paste -sd ' ')
    if [[ $expected != "takes "* ]]; then
      echo "$name: should fail saying '$expected', and passed, taking: $took"
      failed=1
    elif [ "$expected" != "takes $took" ]; then
      echo "$name: should take ${expected#takes }, and took: $took"
      failed=1
    fi
  elif [[ $expected == "takes "* ]]; then
    echo "$name: should pass, and failed: $said"
    failed=1
  elif [[ $said != "$expected"* ]]; then
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
  expect "a file a patch creates, from a $form" "$source" "$work/patches" \
    "takes drivers/kept.c" drivers/kept.c drivers/made.c
  expect "a file a patch creates that the source has, from a $form" "$grown" \
    "$work/patches" \
    "make.patch creates drivers/made.c, which the kernel's source already has." \
    drivers/kept.c drivers/made.c
  expect "a file a stored patch creates that the source has, from a $form" \
    "$grown" "$work/stored" \
    "make.patch creates drivers/made.c, which the kernel's source already has." \
    drivers/kept.c drivers/made.c
  expect "a file a patch creates that one before it deletes, from a $form" \
    "$grown" "$work/remade" \
    "takes drivers/kept.c drivers/made.c" drivers/kept.c drivers/made.c
  expect "a file a patch creates before one deletes it, from a $form" \
    "$grown" "$work/unmade" \
    "make.patch creates drivers/made.c, which the kernel's source already has." \
    drivers/kept.c drivers/made.c
  expect "a file two patches create that the source has, from a $form" \
    "$grown" "$work/twice" \
    "make.patch creates drivers/made.c, which the kernel's source already has." \
    drivers/kept.c drivers/made.c
  expect "a file no patch creates, from a $form" "$source" "$work/patches" \
    "drivers/gone.c isn't in the kernel's source, and no patch creates it." \
    drivers/kept.c drivers/gone.c
done
exit "$failed"
