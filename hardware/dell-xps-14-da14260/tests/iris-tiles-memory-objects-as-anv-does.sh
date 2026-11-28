# shellcheck shell=bash
# Checks iris, as patched, prefers 4K tiles for every memory object in a
# branch of its own, since anv prefers them for the same image and Chrome
# hands WebGPU's frames from anv to iris as memory objects with no
# modifier. Upstream's line is an else of the shared branch before it,
# which every memory object on Xe2 and later takes, so the line must stand
# alone and not as an else, and the preference must follow it. Both sides
# are read, since the canvas breaks whichever of them drifts: anv must
# still prefer 4K tiles for an image it exports with no modifier, and
# iris must keep the constant-bandwidth branch the patch moved. Mesa's own
# iris is then read too, and must fail at that else, so each run shows the
# check catches the bug, not some other failure. anv is read only as
# patched, since the patch leaves it as Mesa ships it.
# usage: iris-tiles-memory-objects-as-anv-does.sh <iris_resource.c> <anv_image.c>
#          <Mesa's own iris_resource.c>
set -euo pipefail

memoryObject='(!res->mod_info && res->external_format != PIPE_FORMAT_NONE) {'
shadowed="iris prefers 4K tiles for a memory object only when no branch before it matched"
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
# it, that starts with $2, as Mesa lays out a function or a block, so a
# block ends at its brace even when an else follows on the same line.
between() {
  awk -v from="$1" -v to="$2" '
    index($0, from) == 1 && !inside { inside = 1; print; next }
    inside { print }
    inside && index($0, to) == 1 { exit }
  ' "$3"
}

# Each judges one source, printing why and exiting 1 at the first thing
# that would leave the canvas a checkerboard. Every test exits on its own,
# so each reads the same whether set -e holds where it's called or not.
judgeIris() {
  local resource=$1 configure flat branch
  [[ -r $resource ]] || unreadable "$resource can't be read"
  configure=$(between 'iris_resource_configure_main(' '}' "$resource")
  [[ -n $configure ]] || unreadable "iris_resource.c has no iris_resource_configure_main"
  # Read as one line, so an else on the line before the branch shadows it too.
  flat=$(tr -s '[:space:]' ' ' <<<"$configure")
  if [[ $flat == *"else if $memoryObject"* ]]; then
    broken "$shadowed"
  fi
  grep -qxF "   if $memoryObject" <<<"$configure" ||
    unreadable "iris_resource_configure_main has no branch for a memory object as it was written"
  branch=$(between "   if $memoryObject" '   }' <(echo "$configure"))
  grep -qxF '      usage |= ISL_SURF_USAGE_PREFER_4K_ALIGNMENT;' <<<"$branch" ||
    broken "iris doesn't prefer 4K tiles for a memory object"
  grep -qxF '      usage |= ISL_SURF_USAGE_DISABLE_AUX_BIT;' <<<"$branch" ||
    broken "iris compresses a memory object anv wrote uncompressed"
  grep -A1 -xF '   else if (templ->bind & PIPE_BIND_CONST_BW)' <<<"$configure" |
    grep -qxF '      usage |= ISL_SURF_USAGE_DISABLE_AUX_BIT;' ||
    broken "iris lost the constant-bandwidth branch the patch moved"
}

judgeAnv() {
  local image=$1 init exported condition
  [[ -r $image ]] || unreadable "$image can't be read"
  init=$(between 'anv_image_init(' '}' "$image")
  [[ -n $init ]] || unreadable "anv_image.c has no anv_image_init"
  exported=$(between '   if (image->vk.external_handle_types != 0 &&' '   }' <(echo "$init"))
  [[ -n $exported ]] || unreadable "anv_image_init has no branch for an image it exports"
  # The condition's two lines, together and whole, so no third can narrow it.
  condition=$(head -n 2 <<<"$exported")
  [[ $condition == "   if (image->vk.external_handle_types != 0 &&
       image->vk.tiling != VK_IMAGE_TILING_DRM_FORMAT_MODIFIER_EXT) {" ]] ||
    unreadable "anv_image_init's branch for an exported image no longer asks only for no modifier"
  grep -qxF '      isl_extra_usage_flags |= ISL_SURF_USAGE_PREFER_4K_ALIGNMENT;' <<<"$exported" ||
    broken "anv no longer prefers 4K tiles for an image it exports with no modifier"
}

judgeIris "$1"
judgeAnv "$2"
status=0
# shellcheck disable=SC2310 # judgeIris failing is the result sought here, and it exits on its own.
released=$(judgeIris "$3") || status=$?
if ((status == 0)); then
  echo "Mesa's own iris passes too, so this check can't tell the patch is there."
  exit 1
fi
[[ $released == "$shadowed, "* ]] || {
  echo "On Mesa's own iris the check failed for another reason: $released"
  echo "So it can't tell the patch is there."
  exit 1
}
