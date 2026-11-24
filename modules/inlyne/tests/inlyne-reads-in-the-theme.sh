# shellcheck shell=bash
# Runs inside cage: opens a markdown file in inlyne with the account's
# settings, shoots the page once it's drawn and still, and closes it.
# inlyne logs a line after every frame it draws (its settings' debug
# metrics, and INLYNE_LOG), so the shot waits for the frames to stop
# rather than for a time. cage doesn't pass on how this ended, so it
# leaves `<photo>.read` once it has the shot.
#
# Keys aren't pressed here: inlyne-says-the-file-it-shows closes it by
# ZZ through a window, and inlyne's own tests hold every key
# (speaks-vim.patch).
#
# usage: inlyne-reads-in-the-theme.sh <file> <photo>
set -euo pipefail

file=$1
photo=$2
: > said

# Without cage's X server, so an inlyne that can't open a Wayland window
# never draws rather than drawing through X.
INLYNE_LOG=inlyne=trace env -u DISPLAY inlyne view "$file" > said 2>&1 &
reader=$!

# Each frame's line says what the redraw histogram recorded, then names it,
# with whatever colour the log puts between them. The first may be long in
# coming on a loaded machine, so it's waited for until a deadline inside
# cage's minute, however quiet the log is meanwhile.
frames='record.*key.*redraw'
deadline=$((SECONDS + 50))
until grep -q -E "$frames" said; do
  if ((SECONDS >= deadline)); then
    echo "inlyne never drew $file in 50 seconds"
    cat said
    exit 1
  fi
  inotifywait -q -q -t 1 -e modify said || true
done
# Once a second passes with nothing logged, the page is still: a picture
# loaded after the first frame draws another.
while inotifywait -q -q -t 1 -e modify said; do :; done
grim "$photo"

kill "$reader"
wait "$reader" || true
touch "$photo.read"
