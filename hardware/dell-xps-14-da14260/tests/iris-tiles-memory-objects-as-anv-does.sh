# shellcheck shell=bash
# Checks iris, as patched, prefers 4K tiles for every memory object in a
# branch of its own, since anv prefers them for the same image and Chrome
# hands WebGPU's frames from anv to iris as memory objects with no
# modifier. Upstream's line is an else of the shared branch before it,
# which every memory object on Xe2 and later takes, so the line must stand
# alone and not as an else, and the preference must follow it.
# usage: iris-tiles-memory-objects-as-anv-does.sh <iris_resource.c>
set -euo pipefail

resource=$1
memoryObject='(!res->mod_info && res->external_format != PIPE_FORMAT_NONE) {'
fail() {
  echo "iris $1, so WebGPU canvases in Chrome show as a checkerboard."
  exit 1
}

grep -qxF "   if $memoryObject" "$resource" ||
  fail "has no branch of its own for a memory object where this check looks"
if grep -qF "else if $memoryObject" "$resource"; then
  fail "prefers 4K tiles for a memory object only when no branch before it matched"
fi
grep -A2 -F "   if $memoryObject" "$resource" |
  grep -qF 'ISL_SURF_USAGE_PREFER_4K_ALIGNMENT' ||
  fail "doesn't prefer 4K tiles for a memory object"
