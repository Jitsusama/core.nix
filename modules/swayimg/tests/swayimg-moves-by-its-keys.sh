# shellcheck shell=bash
# Runs inside cage: opens pictures in swayimg with the account's settings,
# presses every key it lists that moves, trashes, copies or closes as a
# keyboard would, Neovim's beside the arrows that do as they do, and the
# keys that close nothing, and the launcher's command line's commands, as
# swayimg-command carries them, and fails unless each does what it
# should. swayimg writes to `shown` after every frame the picture it has
# drawn or chosen, the mode it's in, where the picture is
# and whether it's full screen. cage doesn't pass on how this ended, so it
# leaves `moved` once every press has.
# usage: swayimg-moves-by-its-keys.sh (beside a folder, pictures, holding
# a.ppm to n.ppm and z-tall.pgm, a picture taller than the window, with a
# wl-copy on nix.lua's path that writes what it's given to ./copied)
#
# What the test writes stays out of the pictures' folder: swayimg reads
# ahead through the folder's files, and one read as it's written, such as
# its own log, stops it with a bus error.
set -euo pipefail

# swayimg lists the folder's other files too, as a folder of downloads
# would have them. Home and End must still reach the first and last
# picture, past a file before them that isn't one.
echo "not a picture" > pictures/0-first.txt
: > shown

# What each frame drawn writes to `shown`.
watched='swayimg.on_redrawn(function()
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
end)'
swayimg --execute="$watched" pictures/a.ppm pictures/b.ppm pictures/c.ppm > said 2>&1 &
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
  if [ -e pictures/d.ppm ] || [ ! -e "$trash/files/d.ppm" ]; then
    echo "$key didn't move d.ppm to the trash: $(ls -A pictures "$trash/files")"
    exit 1
  fi
  typed u d.ppm
  if [ ! -e pictures/d.ppm ] || [ -n "$(find "$trash/files" "$trash/info" -mindepth 1)" ]; then
    echo "u didn't bring d.ppm back: $(ls -A pictures "$trash/files" "$trash/info")"
    exit 1
  fi
done

# The launcher's command line, as swayimg-command carries it: a number goes
# to that picture, and :d moves it to the trash, for u to bring back.
typed gg a.ppm
swayimg-command "$viewer" goto 4
until_shown d.ppm ":4"
swayimg-command "$viewer" delete
until_not_shown d.ppm ":d"
if [ -e pictures/d.ppm ] || [ ! -e "$trash/files/d.ppm" ]; then
  echo ":d didn't move d.ppm to the trash: $(ls -A pictures "$trash/files")"
  exit 1
fi
typed u d.ppm

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

# :rotate turns the picture, which :y copies as it's shown: the tall one,
# 4 by 2000, comes out 2000 by 4, as its PNG's header says. :rotate 270
# turns it back.
press End z-tall.pgm
swayimg-command "$viewer" rotate 90
swayimg-command "$viewer" yank
copied ":y"
size=$(od -An -tu1 -j16 -N8 copied | tr -s ' ' ' ')
if [ "$size" != " 0 0 7 208 0 0 0 4" ]; then
  echo ":rotate and :y gave a picture of$size, not 2000 by 4"
  exit 1
fi
rm copied
swayimg-command "$viewer" rotate 270

# ? asks the launcher for the keys, and Space ? as well. A keyboard sends ?
# as question with Shift held, not as the slash it's printed beside.
asks() {
  rm -f "$ASKED"
  wtype "$@" -M shift -k question -m shift
  for _ in $(seq 100); do
    if grep -qx 'collection shortcuts' "$ASKED" 2>/dev/null; then return; fi
    sleep 0.1
  done
  echo "${*:+Space }? asked the launcher for no keys"
  exit 1
}
asks
asks -k space

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

# Escape leaving full screen isn't pressed here: cage never grants a
# window full screen, so swayimg never is.

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
  if cmp -s copied pictures/a.ppm; then break; fi
  sleep 0.1
done
cmp -s copied pictures/a.ppm || { echo "yy in the gallery didn't copy the file"; exit 1; }
asks

# Escape at rest and q close nothing, in the gallery and on a picture: a
# second on, swayimg is still there. ZZ closes it.
stays_open() {
  if [ "$1" = q ]; then wtype q; else wtype -k "$1"; fi
  sleep 1
  if ! kill -0 "$viewer" 2> /dev/null; then
    echo "$1 closed swayimg $2"
    exit 1
  fi
}
stays_open Escape "in the gallery"
stays_open q "in the gallery"
press Return "in viewer"
stays_open Escape "on a picture"
stays_open q "on a picture"
# Up to ten seconds for swayimg to close.
closes() {
  for _ in $(seq 100); do
    if ! kill -0 "$viewer" 2> /dev/null; then return 0; fi
    sleep 0.1
  done
  echo "$1 didn't close swayimg"
  exit 1
}
wtype -M shift zz -m shift
closes ZZ

# The picture it opened with was said to the desktop's list of files opened
# lately, once, and none it was moved to after.
told=$(if [ -e "$SAID" ]; then tr '\n' ' ' < "$SAID"; else echo 'nothing said'; fi)
if [ "$told" != "add swayimg $PWD/pictures/a.ppm " ]; then
  echo "swayimg said: $told"
  exit 1
fi

# Space w closes one opened afresh, once it has the keyboard, which l
# moving it on shows.
: > shown
swayimg --execute="$watched" pictures/a.ppm pictures/b.ppm > said 2>&1 &
viewer=$!
until_shown a.ppm "opening again" 60
for _ in 1 2 3 4 5; do
  wtype l
  inotifywait -q -q -t 3 -e close_write shown || true
  if grep -q -x b.ppm shown; then
    break
  fi
done
until_shown b.ppm "l, opened again"
wtype -k space w
closes "Space w"

# A command swayimg doesn't have, and :open with a program that isn't here,
# leave it open, and :q closes it. What the last one showed is cleared, so
# the wait is for this one's first frame, after which it hears USR1.
: > shown
swayimg --execute="$watched" pictures/a.ppm > said 2>&1 &
viewer=$!
until_shown a.ppm "opening a third time" 60
swayimg-command "$viewer" nonsense
swayimg-command "$viewer" open missing-program
sleep 1
kill -0 "$viewer" 2> /dev/null || { echo "a command it hasn't closed swayimg"; exit 1; }
swayimg-command "$viewer" quit
closes ":q"

# :open and a program opens the picture in it, through niri, stood in, and
# closes swayimg, as e does.
: > shown
swayimg --execute="$watched" pictures/a.ppm > said 2>&1 &
viewer=$!
until_shown a.ppm "opening a fourth time" 60
swayimg-command "$viewer" open other-editor
closes ":open other-editor"
grep -qxF "msg action spawn -- other-editor $PWD/pictures/a.ppm" "$SPAWNED" \
  || { echo ":open other-editor asked niri for: $(cat "$SPAWNED")"; exit 1; }
touch moved
