# shellcheck shell=bash
# Presses zathura's keys in a compositor of its own and reads back, over
# zathura's D-Bus service, the page each one leaves it on. Run inside cage
# with a session bus, as `keys-test.sh <document of three pages>`, and a
# stand-in launcher-say on the path that writes what it was asked to
# ./asked. It leaves ./passed once every key went where the launcher says it
# goes, since cage ends 0 whatever the script inside it does.
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
until wtype -k End && waits 300 on 2; do
  ends=$((ends + 1))
  if [ "$ends" -ge 6 ]; then never "End to the last page"; fi
done

# Tab goes back to where the jump came from and again returns, Left turns
# back a page and Right on again.
press() {
  wtype -k "$1"
  until_true "$1 to page $2" on "$2"
}
press Tab 0
press Tab 2
press Left 1
press Right 2

# ? asks the launcher for the keys, where it lists the viewer's first.
wtype '?'
asked() { [ "$(cat asked 2>/dev/null)" = "collection shortcuts" ]; }
until_true "? to ask the launcher for its keys" asked

if grep -E '^(warning|error):' said; then exit 1; fi
touch passed
