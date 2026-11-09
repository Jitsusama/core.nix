# shellcheck shell=bash
# The drawings the-launcher-lists-inkscapes-drawings-lately reads, made in
# $HOME/Drawings, and the desktop's list of files opened lately naming
# them at times before $NOW, as GLib writes it, with one more added by a
# program as Inkscape adds one. Each drawing has its root where an SVG's
# is: over lines and after a comment, a compressed one, an export and one
# nvim alone opened; gone.svg is on the list but not here.
# usage: drawings-lately.sh LIST RECENT_FILES, LIST the bookmark file
# with its @NAME@ times to fill, RECENT_FILES the desktop's recent-files.
set -euo pipefail

list=$1
recent_files=$2

mkdir -p "$HOME/Drawings" "$XDG_DATA_HOME"
cd "$HOME/Drawings"
printf '%s\n' '<?xml version="1.0"?>' '<!-- <svg> -->' '<svg' '  width="210mm"' \
  '  height="297mm">' '</svg>' > logo.svg
echo '<svg width="100%" height="100%" viewBox="0 0 1920 1080"></svg>' > 'Tom & Jerry.svg'
echo '<svg viewBox="0,0,48,32"></svg>' | gzip > icon.svgz
echo '<svg width="8.5in" height="280mm"></svg>' > new.svg
echo '<svg viewBox="0 0 1 1"></svg>' > poster.svg
printf PNG > export.png
cd - > /dev/null

# A moment SECONDS before $NOW, to the microsecond as GLib keeps it.
at() { date -u -d "@$((NOW - $1))" "+%Y-%m-%dT%H:%M:%S.$2Z"; }
sed -e "s|@HOME@|$HOME|g" -e "s|@LOGO@|$(at 750 250000)|g" -e "s|@TOM@|$(at 3000 500000)|g" \
  -e "s|@LATER@|$(at 60 000001)|g" -e "s|@ICON@|$(at 172800 750000)|g" \
  -e "s|@GONE@|$(at 100 100000)|g" -e "s|@EXPORT@|$(at 1000 125000)|g" \
  "$list" > "$XDG_DATA_HOME/recently-used.xbel"
# A program adding to it, as Inkscape does, which GLib writes whole.
"$recent_files" add org.inkscape.Inkscape "$HOME/Drawings/new.svg"
