# shellcheck shell=bash
# Stands in for Inkscape where the launcher opens a new drawing in it
# (desktop-keeps-copies-vm-test.py), since Inkscape's window would take
# the keys the walk goes on with: it says the files it was given in the
# runtime folder's inkscape-given, a line each, and adds each to the
# desktop's list of files opened lately under Inkscape's own name, as
# Inkscape does as it opens one, so Inkscape's Recent lists it.
#
# usage: inkscape FILE...
set -euo pipefail

printf '%s\n' "$@" > "$XDG_RUNTIME_DIR/inkscape-given.part"
recent-files add org.inkscape.Inkscape "$@"
mv "$XDG_RUNTIME_DIR/inkscape-given.part" "$XDG_RUNTIME_DIR/inkscape-given"
