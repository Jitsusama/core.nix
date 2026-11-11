# shellcheck shell=bash
# Prints what Inkscape's Recent in the launcher says of each drawing
# Inkscape opened lately (InkscapeProgram.qml), which the programs daemon
# runs as Inkscape is stepped into and holds for it, so the shell reads no
# file and starts no process for it. The desktop's list of files opened
# lately, which Inkscape adds to as it opens and saves each, names them,
# read through GLib (recent-files by) under both names Inkscape adds what
# it opens under, as its own Open Recent reads them back
# (src/ui/desktop/menubar.cpp): its application id, which it sets as
# GLib's program name, and its command. A drawing is an SVG, plain or
# compressed, by its name or its kind.
#
# Newest first, a line each for those still there and at most MOST of
# them: the drawing's whole path, when Inkscape last opened it in
# microseconds since 1970, its weight in bytes and its root element's
# start tag, a tab between each, which the launcher reads the drawing's
# size from (Inkscape.js's extent). When is to the microsecond, as GLib
# keeps it, where the list programs share says it to the second, so
# Inkscape's own row of a drawing is the newer, and the one Recent keeps
# (Recent.js's listed).
#
# Only the start of each is read, where an SVG's root element is, so a
# drawing holding large pictures costs no more than a small one; a
# compressed one is read through gzip. A root element past that start, or
# none, leaves its tag empty, and the drawing is listed without its size.
#
# usage: inkscape-drawings MOST
set -euo pipefail
shopt -s nocasematch

most=$1
listed=0
# How much of each drawing is read for its root element.
start=65536
# Read whole first, so a list GLib can't read fails this rather than
# leaving Recent empty without a word.
had=$(recent-files by org.inkscape.Inkscape inkscape)

while IFS=$'\t' read -r path at kind; do
  [ "$listed" -lt "$most" ] || break
  [ -n "$path" ] || continue
  if [[ $path == *.svgz || $kind == image/svg+xml-compressed ]]; then
    read=(gzip -d -c --)
  elif [[ $path == *.svg || $kind == image/svg+xml ]]; then
    read=(cat --)
  else
    continue
  fi
  [ -f "$path" ] || continue
  size=$(stat -L -c %s -- "$path")
  # Reading stops at the start's end, which closes what reads it early.
  tag=$({ "${read[@]}" "$path" 2> /dev/null || true; } | head -c "$start" \
    | tr '\t\n\r' '   ' | grep -o -E '<svg[[:space:]][^>]*>' | head -n 1 || true)
  printf '%s\t%s\t%s\t%s\n' "$path" "$at" "$size" "$tag"
  listed=$((listed + 1))
done <<< "$had"
