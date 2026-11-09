# shellcheck shell=bash
# Prints what Inkscape's Recent in the launcher says of each drawing
# Inkscape opened lately (InkscapeProgram.qml), given newest first as the
# desktop's list of files opened lately names them (Inkscape.js's
# opened), a line each for those still there and at most MOST of them:
# the drawing's whole path, its weight in bytes and its root element's
# start tag, a tab between each, which the launcher reads the drawing's
# size from (Inkscape.js's extent). A drawing that's gone isn't listed,
# since it can't be opened.
#
# Only the start of each is read, where an SVG's root element is, so a
# drawing holding large pictures costs no more than a small one; a
# compressed one, an .svgz, is read through gzip. A root element past
# that start, or none, leaves its tag empty, and the drawing is listed
# without its size.
#
# usage: inkscape-drawings MOST PATH...
set -euo pipefail

most=$1
shift
listed=0
# How much of each drawing is read for its root element.
start=65536

for path in "$@"; do
  [ "$listed" -lt "$most" ] || break
  [ -f "$path" ] || continue
  size=$(stat -L -c %s -- "$path")
  case $path in
    *.svgz | *.SVGZ) read=(gzip -d -c --) ;;
    *) read=(cat --) ;;
  esac
  # Reading stops at the start's end, which closes what reads it early.
  tag=$({ "${read[@]}" "$path" 2> /dev/null || true; } | head -c "$start" \
    | tr '\t\n\r' '   ' | grep -o -E '<svg[[:space:]][^>]*>' | head -n 1 || true)
  printf '%s\t%s\t%s\n' "$path" "$size" "$tag"
  listed=$((listed + 1))
done
