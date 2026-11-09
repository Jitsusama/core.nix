# shellcheck shell=bash
# The drawings the-launcher-lists-inkscapes-drawings-lately reads, made in
# $HOME/Drawings, and the desktop's list of files opened lately naming
# them at times before NOW, as GLib writes it, with one more added by a
# program as Inkscape adds one. Each drawing has its root where an SVG's
# is: over lines and after a comment, a compressed one, an export and one
# nvim alone opened; gone.svg is on the list but not here.
# usage: drawings-lately.sh LIST RECENT_FILES, LIST the bookmark file
# with its @NAME@ times to fill, RECENT_FILES the desktop's recent-files.
set -euo pipefail

list=$1
recent_files=$2
now=${NOW:?the moment the check runs at, in seconds}
data=${XDG_DATA_HOME:?where the desktop keeps its list}

mkdir -p "$HOME/Drawings" "$data"
cd "$HOME/Drawings"
printf '%s\n' '<?xml version="1.0"?>' '<!-- <svg> -->' '<svg' '  width="210mm"' \
  '  height="297mm">' '</svg>' > logo.svg
echo '<svg width="100%" height="100%" viewBox="0 0 1920 1080"></svg>' > 'Tom & Jerry.svg'
echo '<svg viewBox="0,0,48,32"></svg>' | gzip > icon.svgz
echo '<svg width="8.5in" height="280mm"></svg>' > new.svg
echo '<svg viewBox="0 0 1 1"></svg>' > poster.svg
printf PNG > export.png
cd - > /dev/null

# A moment SECONDS before now, to the microsecond as GLib keeps it.
at() { date -u -d "@$((now - $1))" "+%Y-%m-%dT%H:%M:%S.$2Z"; }
logo=$(at 750 250000)
tom=$(at 3000 500000)
later=$(at 60 000001)
icon=$(at 172800 750000)
gone=$(at 100 100000)
exported=$(at 1000 125000)
sed -e "s|@HOME@|$HOME|g" -e "s|@LOGO@|$logo|g" -e "s|@TOM@|$tom|g" \
  -e "s|@LATER@|$later|g" -e "s|@ICON@|$icon|g" \
  -e "s|@GONE@|$gone|g" -e "s|@EXPORT@|$exported|g" \
  "$list" > "$data/recently-used.xbel"
# A program adding to it, as Inkscape does, which GLib writes whole.
"$recent_files" add org.inkscape.Inkscape "$HOME/Drawings/new.svg"
