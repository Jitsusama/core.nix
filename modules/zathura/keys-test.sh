# shellcheck shell=bash
# Presses every key zathura lists in a compositor of its own and checks
# where each lands: a key that turns the page by the page zathura says it
# is on over its D-Bus service, and one that scrolls by what the screen
# shows, each arrow beside its letter, and the keys that close it beside q,
# which doesn't. Run inside cage with a session
# bus, as `keys-test.sh <document of six empty pages, with "find" on the
# fourth and the sixth, named b.pdf in a folder beside a.pdf and c.pdf>`,
# a stand-in launcher-say on the path that writes what it was asked to
# ./asked, and zathura-command on the path. It leaves ./passed once every key
# went where the launcher says it goes, since cage ends 0 whatever the
# script inside it does.
set -eu

zathura "$1" > said 2>&1 &
zathura=$!
trap 'kill $zathura 2>/dev/null || true' EXIT

# Waits for a command to succeed, trying it that many times a tenth of a
# second apart.
waits() {
  tries=$1
  shift
  for _ in $(seq "$tries"); do
    if "$@"; then return 0; fi
    sleep 0.1
  done
  return 1
}

# Waits up to two minutes for a command to succeed, saying what it waited
# for when it never does. A busy host can take many seconds to draw a page,
# so the wait is a ceiling, and only one never met says something's wrong.
never() {
  echo "never: $1" >&2
  cat said >&2
  exit 1
}
until_true() {
  what=$1
  shift
  waits 1200 "$@" || never "$what"
}

bus=""
named() {
  bus=$(gdbus call --session --dest org.freedesktop.DBus \
    --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.ListNames |
    grep -o "org\.pwmt\.zathura\.PID-$zathura" || true)
  [ -n "$bus" ]
}
until_true "zathura on the bus" named

page() {
  gdbus call --session --dest "$bus" --object-path /org/pwmt/zathura \
    --method org.freedesktop.DBus.Properties.Get org.pwmt.zathura pagenumber |
    grep -o -E '[0-9]+' | tail -1
}
on() { [ "$(page)" = "$1" ]; }
until_true "the first page" on 0

# zathura is on the bus before its window has the keyboard, and a key sent
# before then goes nowhere. End jumps to the last page and, pressed again,
# stays there, so it's pressed until it lands; every key after it does.
ends=0
until wtype -k End && waits 300 on 5; do
  ends=$((ends + 1))
  if [ "$ends" -ge 6 ]; then never "End to the last page"; fi
done

# A key by its name, or Ctrl and a letter, then the page it lands on.
press() {
  wtype -k "$1"
  until_true "$1 to page $2" on "$2"
}
held() {
  wtype -M ctrl -k "$1" -m ctrl
  until_true "Ctrl+$1 to page $2" on "$2"
}
# What's typed, as Neovim's keys are, then the page it lands on.
typed() {
  wtype "$1"
  until_true "$1 to page $2" on "$2"
}

# Ctrl+O goes back through the jumps and Ctrl+I forward again, and Tab
# goes back to where the jump came from and again returns.
held o 0
held i 5
press Tab 0
press Tab 5

# K turns back a page and J on again, as Page Up and Page Down do.
typed K 4
typed J 5
press Page_Up 4
press Page_Down 5

# Home and End, gg and G, go to the first and the last, and a count before
# G to that page.
press Home 0
press End 5
typed gg 0
typed G 5
typed 3G 2

# m and a letter marks a place, and ' and the letter goes back to it.
wtype ma
typed gg 0
typed "'a" 2

# A scroll moves the pages less than a page, so it's seen on the screen,
# where the empty pages and the gaps between them stand at a new height.
# The screen is still once two looks half a second apart agree, which an
# empty page still being drawn passes too, since it's drawn the colour of
# one that's done.
looked() { grim -t ppm "$1"; }
still() {
  looked "$1"
  for _ in $(seq 240); do
    sleep 0.5
    looked "$1.next"
    if cmp -s "$1" "$1.next"; then
      rm "$1.next"
      return 0
    fi
    mv "$1.next" "$1"
  done
  never "the screen to be still for $1"
}
alike() {
  looked now
  cmp -s now "$1"
}
moved() {
  looked now
  ! cmp -s now "$1"
}
shows() {
  what=$1
  shift
  until_true "$what" "$@"
}

press Home 0
still top
# Down scrolls a little, and j as far.
wtype -k Down
shows "Down to scroll" moved top
still down
press Home 0
shows "Home back to the top" alike top
wtype j
shows "j to scroll as far as Down" alike down
# Up scrolls back, and k as far.
wtype -k Up
shows "Up back to the top" alike top
wtype j
shows "j again" alike down
wtype k
shows "k back to the top, as Up" alike top

# Ctrl+D and Ctrl+U go half a screen, twice a screen, and Ctrl+F and
# Ctrl+B a screen, which takes the middle of the screen to the next page.
wtype -M ctrl -k d -m ctrl
shows "Ctrl+d to scroll" moved top
shows "Ctrl+d further than j" moved down
wtype -M ctrl -k u -m ctrl
shows "Ctrl+u back to the top" alike top
wtype -M ctrl -k d -m ctrl
held d 1
wtype -M ctrl -k u -m ctrl
held u 0
shows "Ctrl+u twice back to the top" alike top
held f 1
held b 0
shows "Ctrl+b back to the top" alike top

# h and l scroll sideways, on pages wider than the window: two pages
# side by side, d, then their width in view, s, and then larger, so the
# gap between them moves as the pages do.
wtype d
wtype s
wtype '='
for _ in $(seq 20); do wtype h; done
still left
wtype l
shows "l to scroll right" moved left
still right
wtype h
shows "h back to the left" alike left
# Right and Left scroll as l and h do, and turn no page.
wtype -k Right
shows "Right to scroll as far as l" alike right
wtype -k Left
shows "Left back to the left, as h" alike left
wtype d
wtype a

# / finds words on from the page it's on and ? back from it, n goes on to
# the next and N back to the one before. Each search ends on Ctrl+J,
# zathura's Return in its command line: GTK hears Return there through a
# keymap it read before wtype sent another, so it misses it here, where
# Ctrl+J is read afresh.
found() {
  wtype "$1" -M ctrl -k j -m ctrl
  until_true "$1 to page $2" on "$2"
}
typed gg 0
found /find 3
typed n 5
typed N 3
press End 5
typed K 4
found '?find' 3

# Space ? asks the launcher for the keys, where it lists the viewer's
# first.
wtype -k space '?'
asked() { [ "$(cat asked 2>/dev/null)" = "collection shortcuts" ]; }
until_true "Space ? to ask the launcher for its keys" asked

# : asks the launcher for its command line over this window, by its bus
# name, named for the document.
wtype :
commanded() { [ "$(cat asked 2>/dev/null)" = "command-line zathura $bus b.pdf" ]; }
until_true ": to ask the launcher for its command line" commanded

# What the launcher's line chooses comes back through zathura-command:
# :3 to that page, :$ to the last and a page past it to the last too,
# and zathura's own commands as its own line would run them, a bookmark
# made on one page and gone back to from another, and a command zathura
# doesn't have refused.
carried() {
  zathura-command run "$bus" "$1" "$2"
  until_true ":$1 $2 to page $3" on "$3"
}
carried goto 3 2
carried bmark kept 2
carried goto '$' 5
carried goto 1 0
carried bjump kept 2
carried goto 99 5
if zathura-command run "$bus" nonsense "" 2>/dev/null; then never "a command zathura lacks refused"; fi

# ]b opens the next document in its folder, by name, and [b the one before,
# with a count that many, coming round at either end, and passing over
# what zathura doesn't open.
has_open() {
  gdbus call --session --dest "$bus" --object-path /org/pwmt/zathura \
    --method org.freedesktop.DBus.Properties.Get org.pwmt.zathura filename |
    grep -q "/$1'"
}
opened() {
  wtype "$1"
  until_true "$1 to open $2" has_open "$2"
}
opened ']b' c.pdf
opened '[b' b.pdf
opened '2]b' a.pdf
opened '2[b' b.pdf

# q closes nothing: a second on, zathura is still there. ZZ closes it.
wtype q
sleep 1
kill -0 "$zathura" 2>/dev/null || never "q to leave it open"
wtype ZZ
gone() { ! kill -0 "$zathura" 2>/dev/null; }
until_true "ZZ to close" gone
if grep -E '^(warning|error):' said; then exit 1; fi

# Space w closes one opened afresh, once Home and End landing show it has
# the keyboard: it opens where the last one left the document, so either
# alone might land where it already was.
zathura "$1" > said 2>&1 &
zathura=$!
until_true "zathura on the bus again" named
ends=0
until wtype -k Home && waits 300 on 0 && wtype -k End && waits 300 on 5; do
  ends=$((ends + 1))
  if [ "$ends" -ge 6 ]; then never "End to the last page, opened again"; fi
done
wtype -k space w
until_true "Space w to close" gone

if grep -E '^(warning|error):' said; then exit 1; fi
touch passed
