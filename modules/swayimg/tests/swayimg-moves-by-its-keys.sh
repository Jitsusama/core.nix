# shellcheck shell=bash
# Runs inside cage: opens three pictures in swayimg with the account's
# settings, presses the keys that move between them as a keyboard would,
# and fails unless each press shows the picture it should. swayimg writes
# the picture it has drawn to `shown` after every frame. cage doesn't pass
# on how this ended, so it leaves `moved` once every press has.
# usage: swayimg-moves-by-its-keys.sh (in a folder holding a.ppm, b.ppm
# and c.ppm)
set -euo pipefail

# swayimg lists the folder's other files too, as a folder of downloads
# would have them. Home and End must still reach the first and last
# picture, past a file before them that isn't one, and `shown` after them.
echo "not a picture" > 0-first.txt
: > shown

swayimg --execute='swayimg.on_redrawn(function()
  local image = swayimg.viewer.get_image()
  local file = io.open("shown", "w")
  file:write(image and image.path:match("[^/]+$") or "", "\n")
  file:close()
end)' a.ppm b.ppm c.ppm > said 2>&1 &
viewer=$!

# Until swayimg has drawn the picture wanted, or so many seconds pass with
# no frame drawn: ten, or as many as the third word says. A frame drawn
# between the look and the wait is looked at once more before giving up.
until_shown() {
  touch shown
  until grep -q -x "$1" shown; do
    if ! inotifywait -q -q -t "${3:-10}" -e close_write shown && ! grep -q -x "$1" shown; then
      echo "swayimg showed $(cat shown) where $2 should show $1"
      cat said
      exit 1
    fi
  done
}

# The first frame waits on cage and swayimg starting, which on a machine
# busy with other builds took past ten seconds (2026-10-29), so it's given
# a minute; a press, once both are running, keeps its ten.
until_shown a.ppm "opening the first" 60
# cage gives swayimg the keyboard a moment after its first frame, and a key
# pressed before that goes nowhere, so the first is pressed until it lands.
# swayimg can't say when it has the keyboard; a press it hears moves it
# once, and each press waits three seconds for the frame that shows it.
for _ in 1 2 3 4 5; do
  wtype -k Right
  inotifywait -q -q -t 3 -e close_write shown || true
  if grep -q -x b.ppm shown; then
    break
  fi
done
until_shown b.ppm "→"
wtype -k Left
until_shown a.ppm "←"
wtype -k End
until_shown c.ppm "End"
wtype -k Home
until_shown a.ppm "Home"

wtype q
wait "$viewer"

# The picture it opened with was said to the desktop's list of files opened
# lately, once, and none it was moved to after.
told=$(if [ -e "$SAID" ]; then tr '\n' ' ' < "$SAID"; else echo 'nothing said'; fi)
if [ "$told" != "add swayimg $PWD/a.ppm " ]; then
  echo "swayimg said: $told"
  exit 1
fi
touch moved
