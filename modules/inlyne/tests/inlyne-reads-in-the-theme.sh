# shellcheck shell=bash
# Runs inside cage: opens a markdown file in inlyne with the account's
# settings, photographs the page once it's drawn and still, and closes it.
# inlyne logs a line after every frame it draws (its settings' debug
# metrics, and INLYNE_LOG), so the photograph waits for the frames to stop
# rather than for a time. cage doesn't pass on how this ended, so it
# leaves `<photo>.read` once it has the photograph.
#
# Keys aren't pressed: inlyne's window library reads a Wayland key by its
# code, and wtype's virtual keyboard puts every key on codes of its own,
# so each arrives as another key. inlyne's own tests hold its keys
# (keys-page.patch).
#
# usage: inlyne-reads-in-the-theme.sh <file> <photo>
set -euo pipefail

file=$1
photo=$2
: > said

INLYNE_LOG=inlyne=trace inlyne view "$file" > said 2>&1 &
reader=$!

# Each frame's line says what the redraw histogram recorded, then names it,
# with whatever colour the log puts between them.
frames='record.*key.*redraw'
until grep -q -E "$frames" said; do
  if ! inotifywait -q -q -t 10 -e modify said && ! grep -q -E "$frames" said; then
    echo "inlyne never drew $file"
    cat said
    exit 1
  fi
done
# Once a second passes with nothing logged, the page is still: a picture
# loaded after the first frame draws another.
while inotifywait -q -q -t 1 -e modify said; do :; done
grim "$photo"

kill "$reader"
wait "$reader" || true
touch "$photo.read"
