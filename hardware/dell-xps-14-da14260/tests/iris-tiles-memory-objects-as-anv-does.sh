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
# The source no longer reads as this check expects, which says nothing
# yet about the canvas: read the function again and fit the check to it.
unreadable() {
  echo "$1, so this check can't tell how a memory object is tiled."
  exit 1
}
broken() {
  echo "$1, so WebGPU canvases in Chrome show as a checkerboard."
  exit 1
}

# Prints the lines from the first that starts with $1 to the first, after
# it, that is exactly $2, as Mesa lays out a function or a block.
between() {
  awk -v from="$1" -v to="$2" '
    index($0, from) == 1 { inside = 1 }
    inside { print }
    inside && $0 == to { exit }
  ' "$3"
}

configure=$(between 'iris_resource_configure_main(' '}' "$resource")
[[ -n $configure ]] || unreadable "iris_resource.c has no iris_resource_configure_main"
if grep -qF "else if $memoryObject" <<<"$configure"; then
  broken "iris prefers 4K tiles for a memory object only when no branch before it matched"
fi
grep -qxF "   if $memoryObject" <<<"$configure" ||
  unreadable "iris_resource_configure_main has no branch for a memory object as it was written"
branch=$(grep -A3 -xF "   if $memoryObject" <<<"$configure")
grep -qxF '      usage |= ISL_SURF_USAGE_PREFER_4K_ALIGNMENT;' <<<"$branch" ||
  broken "iris doesn't prefer 4K tiles for a memory object"
grep -qxF '      usage |= ISL_SURF_USAGE_DISABLE_AUX_BIT;' <<<"$branch" ||
  broken "iris compresses a memory object anv wrote uncompressed"
grep -A1 -xF '   else if (templ->bind & PIPE_BIND_CONST_BW)' <<<"$configure" |
  grep -qxF '      usage |= ISL_SURF_USAGE_DISABLE_AUX_BIT;' ||
  broken "iris lost the constant-bandwidth branch the patch moved"

init=$(between 'anv_image_init(' '}' "$image")
[[ -n $init ]] || unreadable "anv_image.c has no anv_image_init"
exported=$(between '   if (image->vk.external_handle_types != 0 &&' '   }' <(echo "$init"))
[[ -n $exported ]] || unreadable "anv_image_init has no branch for an image it exports"
grep -qxF '       image->vk.tiling != VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT) {' <<<"$exported" ||
  unreadable "anv_image_init's branch for an exported image no longer asks only for no modifier"
grep -qxF '      isl_extra_usage_flags |= ISL_SURF_USAGE_PREFER_4K_ALIGNMENT;' <<<"$exported" ||
  broken "anv no longer prefers 4K tiles for an image it exports with no modifier"
