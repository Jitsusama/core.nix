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

# Waits up to ten seconds for a command to succeed, saying what it waited
# for when it never does.
until_true() {
  what=$1
  shift
  for _ in $(seq 100); do
    if "$@"; then return 0; fi
    sleep 0.1
  done
  echo "never: $what" >&2
  cat said >&2
  exit 1
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

# Right turns a page, End jumps to the last, Tab goes back to where the
# jump came from and again returns, Left turns back.
press() {
  wtype -k "$1"
  until_true "$1 to page $2" on "$2"
}
press Right 1
press End 2
press Tab 1
press Tab 2
press Left 1

# ? asks the launcher for the keys, where it lists the viewer's first.
wtype '?'
asked() { [ "$(cat asked 2>/dev/null)" = "collection shortcuts" ]; }
until_true "? to ask the launcher for its keys" asked

if grep -E '^(warning|error):' said; then exit 1; fi
touch passed
