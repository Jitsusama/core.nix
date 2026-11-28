# shellcheck shell=bash
# Checks iris, as patched, prefers 4K tiles for every memory object in a
# branch of its own, since anv prefers them for the same image and Chrome
# hands WebGPU's frames from anv to iris as memory objects with no
# modifier. Upstream's line is an else of the shared branch before it,
# which every memory object on Xe2 and later takes, so the line must stand
# alone and not as an else, and the preference must follow it. Both sides
# are read, since the canvas breaks whichever of them drifts: anv must
# still prefer 4K tiles for an image it exports with no modifier, and
# iris must keep the constant-bandwidth branch the patch moved.
# usage: iris-tiles-memory-objects-as-anv-does.sh <iris_resource.c> <anv_image.c>
set -euo pipefail

resource=$1
image=$2
memoryObject='(!res->mod_info && res->external_format != PIPE_FORMAT_NONE) {'
fail() {
  echo "$1, so WebGPU canvases in Chrome show as a checkerboard."
  exit 1
}

# Prints one C function's body, from the line naming it at the start to
# the brace that closes it at the start of a line, as Mesa lays them out.
body() {
  awk -v name="$1(" 'index($0, name) == 1 { inside = 1 } inside { print } inside && /^}/ { exit }' "$2"
}

configure=$(body iris_resource_configure_main "$resource")
[[ -n $configure ]] || fail "iris has no iris_resource_configure_main where this check looks"
grep -qxF "   if $memoryObject" <<<"$configure" ||
  fail "iris has no branch of its own for a memory object where this check looks"
if grep -qF "else if $memoryObject" <<<"$configure"; then
  fail "iris prefers 4K tiles for a memory object only when no branch before it matched"
fi
grep -A2 -xF "   if $memoryObject" <<<"$configure" |
  grep -qF 'ISL_SURF_USAGE_PREFER_4K_ALIGNMENT' ||
  fail "iris doesn't prefer 4K tiles for a memory object"
grep -A1 -xF '   else if (templ->bind & PIPE_BIND_CONST_BW)' <<<"$configure" |
  grep -qF 'ISL_SURF_USAGE_DISABLE_AUX_BIT' ||
  fail "iris lost the constant-bandwidth branch the patch moved"

init=$(body anv_image_init "$image")
[[ -n $init ]] || fail "anv has no anv_image_init where this check looks"
grep -A8 -F 'image->vk.tiling != VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT) {' <<<"$init" |
  grep -qF 'ISL_SURF_USAGE_PREFER_4K_ALIGNMENT' ||
  fail "anv no longer prefers 4K tiles for an image it exports with no modifier"
