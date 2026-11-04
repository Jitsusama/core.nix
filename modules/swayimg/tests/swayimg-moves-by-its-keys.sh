# shellcheck shell=bash
# Runs inside cage: opens pictures in swayimg with the account's settings,
# presses every key it lists that moves, trashes or copies as a keyboard
# would, Neovim's beside the arrows they twin, and fails unless each press
# does what it should. swayimg writes to `shown` after every frame the
# picture it has drawn or chosen, the mode it's in, where the picture is
# and whether it's full screen. cage doesn't pass on how this ended, so it
# leaves `moved` once every press has.
# usage: swayimg-moves-by-its-keys.sh (in a folder holding a.ppm to
# n.ppm and z-tall.pgm, a picture taller than the window, with a wl-copy
# on nix.lua's path that writes what it's given to ./copied)
set -euo pipefail

# swayimg lists the folder's other files too, as a folder of downloads
# would have them. Home and End must still reach the first and last
# picture, past a file before them that isn't one, and `shown` after them.
echo "not a picture" > 0-first.txt
: > shown

swayimg --execute='swayimg.on_redrawn(function()
  local mode = swayimg.mode
  local image, at = nil, { x = 0, y = 0 }
  if mode == "gallery" then
    image = swayimg.gallery.get_image()
  else
    image = swayimg.viewer.get_image()
    if image then
      at = swayimg.viewer.get_position()
    end
  end
  local file = io.open("shown", "w")
  file:write(image and image.path:match("[^/]+$") or "", "\n", "in ", mode, "\n",
    "at ", at.x, " ", at.y, "\n", "full ", tostring(swayimg.fullscreen), "\n")
  file:close()
end)' a.ppm b.ppm c.ppm > said 2>&1 &
viewer=$!

# Until swayimg has drawn what's wanted, a line of `shown`, or so many
# seconds pass with no frame drawn: ten, or as many as the third word says.
# A frame drawn between the look and the wait is looked at once more before
# giving up.
until_shown() {
  touch shown
  until grep -q -x "$1" shown; do
    if ! inotifywait -q -q -t "${3:-10}" -e close_write shown && ! grep -q -x "$1" shown; then
      echo "swayimg showed $(tr '\n' ' ' < shown)where $2 should show $1"
      cat said
      exit 1
    fi
  done
}
# Until it has drawn something other than that.
until_not_shown() {
  touch shown
  while grep -q -x "$1" shown; do
    if ! inotifywait -q -q -t 10 -e close_write shown && grep -q -x "$1" shown; then
      echo "swayimg still showed $1 after $2"
      cat said
      exit 1
    fi
  done
}
# Until a second passes with no frame drawn.
until_still() {
  while inotifywait -q -q -t 1 -e close_write shown; do :; done
}
line() { sed -n "$1p" shown; }

# A named key, what's typed, or a letter with Shift, then what it shows.
press() {
  wtype -k "$1"
  until_shown "$2" "$1"
}
typed() {
  wtype "$1"
  until_shown "$2" "$1"
}
# A number or a key before it goes in the same wtype, since what waits
# goes 300 ms on, which a second wtype starting on a busy machine can miss.
shifted() {
  wtype ${3:+"$3"} -M shift "$1" -m shift
  until_shown "$2" "${3:-}Shift+$1"
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
press Left a.ppm
typed l b.ppm
typed h a.ppm
# ]b and [b, as Vim's go between buffers.
typed ']b' b.ppm
typed '[b' a.ppm

# A number before a move makes it that many.
typed 3l d.ppm
typed 3h a.ppm
# A number goes 300 ms on with nothing more, and Escape stops one rather
# than closing.
wtype 2
sleep 1
typed l b.ppm
typed h a.ppm
wtype 3 -k Escape
typed l b.ppm
typed h a.ppm

# Home and End, gg and G, go to the first and the last, and a number before
# G to that picture, counted as its count is, once the file that isn't one
# has been walked past.
press End z-tall.pgm
press Home a.ppm
shifted g z-tall.pgm
typed gg a.ppm
shifted g d.ppm 4

# dd and Delete move the picture to the trash, and u brings it back.
trash=.local/share/Trash
for key in dd Delete; do
  if [ "$key" = dd ]; then wtype dd; else wtype -k Delete; fi
  until_not_shown d.ppm "$key"
  if [ -e d.ppm ] || [ ! -e "$trash/files/d.ppm" ]; then
    echo "$key didn't move d.ppm to the trash: $(ls -A . "$trash/files")"
    exit 1
  fi
  typed u d.ppm
  if [ ! -e d.ppm ] || [ -n "$(find "$trash/files" "$trash/info" -mindepth 1)" ]; then
    echo "u didn't bring d.ppm back: $(ls -A . "$trash/files" "$trash/info")"
    exit 1
  fi
done

# yy copies the picture as shown, a PNG, and y alone waits for what it's
# to copy, as in Vim, and copies nothing.
copied() {
  for _ in $(seq 100); do
    if [ -s copied ] && [ "$(head -c 4 copied | tail -c 3)" = PNG ]; then return 0; fi
    sleep 0.1
  done
  echo "$1 didn't copy the picture as a PNG"
  exit 1
}
wtype yy
copied yy
rm copied
wtype y
sleep 1
if [ -e copied ]; then
  echo "y alone copied the picture"
  exit 1
fi

# Space ? lists the keys over the picture, as ? does, and again puts them
# away. The list is only drawn, so it's seen on the screen, once the words
# a picture opens with have gone, two seconds on.
sleep 3
until_still
grim -t ppm bare.ppm
wtype -k space -M shift -k slash -m shift
listed=0
for _ in $(seq 100); do
  grim -t ppm now.ppm
  if ! cmp -s bare.ppm now.ppm; then listed=1 && break; fi
  sleep 0.1
done
[ "$listed" = 1 ] || { echo "Space ? listed no keys"; exit 1; }
wtype -k space -M shift -k slash -m shift

# On a picture larger than the window, Down moves it and Up back, and j and
# k as far.
press End z-tall.pgm
for _ in $(seq 12); do wtype '='; done
until_still
top=$(line 3)
wtype -k Down
until_not_shown "$top" "Down"
until_still
down=$(line 3)
press Up "$top"
typed j "$down"
typed k "$top"

# Escape leaving full screen before it closes isn't pressed here: cage
# never grants a window full screen, so swayimg never is.

# In the gallery, h j k l choose as the arrows do, gg and G go to the
# first and the last, a number before G to that picture, and a number
# before a move makes it that many. yy copies the file.
press Return "in gallery"
typed gg a.ppm
typed l b.ppm
typed h a.ppm
typed ']b' b.ppm
typed '[b' a.ppm
typed 3l d.ppm
shifted g z-tall.pgm
shifted g b.ppm 2
typed gg a.ppm
wtype -k Down
until_not_shown a.ppm "Down in the gallery"
until_still
below=$(line 1)
typed gg a.ppm
typed j "$below"
typed k a.ppm
wtype yy
for _ in $(seq 100); do
  if cmp -s copied a.ppm; then break; fi
  sleep 0.1
done
cmp -s copied a.ppm || { echo "yy in the gallery didn't copy the file"; exit 1; }

# Escape at rest closes it.
wtype -k Escape
wait "$viewer"

# The picture it opened with was said to the desktop's list of files opened
# lately, once, and none it was moved to after.
told=$(if [ -e "$SAID" ]; then tr '\n' ' ' < "$SAID"; else echo 'nothing said'; fi)
if [ "$told" != "add swayimg $PWD/a.ppm " ]; then
  echo "swayimg said: $told"
  exit 1
fi
touch moved
